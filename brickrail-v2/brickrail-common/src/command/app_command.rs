use std::collections::VecDeque;

use bevy::prelude::*;

use crate::block::Block;
use crate::connection::Connection;
use crate::layout::Layout;
use crate::layout_primitives::{LogicalBlockID, TrainID};
use crate::lifecycle::{ElementId, Registry, SpawnElement};
use crate::marker::Marker;
use crate::route::{RouteLeg, TrainLegs};
use crate::track::Track;
use crate::train::Train;
use crate::train_position::{TrainLegState, TrainPosition};

use super::{
    CommandEnvelope, CommandId, CommandRegistry, CommandResponse, CommandState,
    ExitControlModeRequest, SimulationCommand, SubAppCommandInputQueue,
};

/// Top-level command enum for the client side.
/// Covers both client-local commands and simulation commands forwarded to the SubApp.
#[derive(Component, Clone, Debug)]
pub enum AppCommand {
    /// Spawn layout elements in the main world (for rendering/editing).
    SpawnLayout(Layout),
    /// Set a train's cached position. Inserts `TrainBlockPosition` on the
    /// main-world train entity. Used before `EnterControlMode` so the
    /// simulation auto-places the train.
    SetTrainPosition(TrainID, LogicalBlockID),
    /// Enter control mode: forwards to SubApp with cached train positions.
    /// Trains with `TrainBlockPosition` are auto-placed in the simulation.
    EnterControlMode(Layout),
    /// Exit control mode: extracts train positions into `TrainBlockPosition`
    /// components, then forwards despawn to SubApp.
    ExitControlMode,
    /// Forward a simulation command to the SubApp.
    Simulation(SimulationCommand),
}

/// Cached train block position. Inserted on main-world train entities
/// when exiting control mode, used to restore position on re-enter.
#[derive(Component, Clone, Debug)]
pub struct TrainBlockPosition(pub LogicalBlockID);

/// Domain request: spawn layout elements in the main world.
#[derive(Clone, Debug)]
pub struct SpawnLayoutRequest {
    pub layout: Layout,
}

/// Entity-backed sequential command queue.
/// Callers get an `Entity` handle immediately on push, which they can query
/// for `CommandState` to track progress.
#[derive(Resource, Default)]
pub struct AppCommandQueue {
    queue: VecDeque<Entity>,
    in_flight: Option<Entity>,
}

impl AppCommandQueue {
    /// Push a command onto the queue. Creates the backing entity immediately
    /// and returns it so callers can track its state.
    pub fn push(
        &mut self,
        commands: &mut Commands,
        registry: &mut CommandRegistry,
        command: AppCommand,
    ) -> Entity {
        let cmd_id = registry.next_id();
        let entity = commands
            .spawn((cmd_id, CommandState::Pending, command))
            .id();
        registry.insert(cmd_id, entity);
        self.queue.push_back(entity);
        entity
    }

    /// Push a command using direct World access. Useful in tests.
    pub fn push_world(world: &mut World, command: AppCommand) -> Entity {
        world.resource_scope(|world, mut queue: Mut<AppCommandQueue>| {
            world.resource_scope(|world, mut registry: Mut<CommandRegistry>| {
                let cmd_id = registry.next_id();
                let entity = world.spawn((cmd_id, CommandState::Pending, command)).id();
                registry.insert(cmd_id, entity);
                queue.queue.push_back(entity);
                entity
            })
        })
    }
}

/// Client-side command dispatch plugin.
/// Registers the `AppCommandQueue` and the dispatch + handler systems.
pub struct AppCommandPlugin;

impl Plugin for AppCommandPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppCommandQueue>();
        app.add_message::<CommandEnvelope<SpawnLayoutRequest>>();
        app.add_systems(
            Update,
            (
                dispatch_app_commands,
                handle_spawn_layout.run_if(on_message::<CommandEnvelope<SpawnLayoutRequest>>),
            )
                .chain(),
        );
    }
}

/// Fan-out dispatch: pops the next command from the queue and writes a typed
/// envelope. Client-local commands get typed envelopes handled by systems in
/// this plugin. Simulation commands are forwarded to the SubApp input queue.
fn dispatch_app_commands(
    mut queue: ResMut<AppCommandQueue>,
    entity_query: Query<(&CommandId, &CommandState, &AppCommand)>,
    mut cmd_queue: ResMut<SubAppCommandInputQueue>,
    mut spawn_layout_writer: MessageWriter<CommandEnvelope<SpawnLayoutRequest>>,
    train_query: Query<(Entity, &ElementId<Train>, &TrainPosition, &TrainLegs)>,
    leg_query: Query<&RouteLeg>,
    cached_position_query: Query<(&ElementId<Train>, &TrainBlockPosition)>,
    train_registry: Res<Registry<Train>>,
    mut commands: Commands,
    mut response_writer: MessageWriter<CommandResponse>,
) {
    // Check if the in-flight command has completed.
    if let Some(in_flight) = queue.in_flight {
        match entity_query.get(in_flight) {
            Ok((_, CommandState::Completed | CommandState::Failed(_), _)) => {
                queue.in_flight = None;
            }
            Ok((_, CommandState::Pending, _)) => return,
            Err(_) => {
                queue.in_flight = None;
            }
        }
    }

    // Dispatch the next command.
    let Some(entity) = queue.queue.pop_front() else {
        return;
    };
    let Ok((cmd_id, _, app_command)) = entity_query.get(entity) else {
        return;
    };

    match app_command.clone() {
        AppCommand::SpawnLayout(layout) => {
            spawn_layout_writer.write(CommandEnvelope {
                command_id: *cmd_id,
                request: SpawnLayoutRequest { layout },
            });
        }
        AppCommand::SetTrainPosition(train_id, block) => {
            // Insert TrainBlockPosition on the main-world train entity.
            if let Some(train_entity) = train_registry.get(&train_id) {
                commands
                    .entity(train_entity)
                    .insert(TrainBlockPosition(block));
            }
            // Complete immediately via response.
            response_writer.write(CommandResponse {
                command_id: *cmd_id,
                result: Ok(()),
            });
        }
        AppCommand::EnterControlMode(layout) => {
            // Collect cached train positions and bundle into the request.
            let train_positions: Vec<_> = cached_position_query
                .iter()
                .map(|(id, pos)| (id.0, pos.0))
                .collect();
            cmd_queue.0.push(CommandEnvelope {
                command_id: *cmd_id,
                request: SimulationCommand::EnterControlMode(super::EnterControlModeRequest {
                    layout,
                    train_positions,
                }),
            });
        }
        AppCommand::ExitControlMode => {
            // Extract train positions from main-world mirror before despawn.
            for (train_entity, _train_id, position, legs) in &train_query {
                if position.leg_state == TrainLegState::EnteredTarget {
                    if let Some(&leg_entity) = legs.collection().first() {
                        if let Ok(leg) = leg_query.get(leg_entity) {
                            commands
                                .entity(train_entity)
                                .insert(TrainBlockPosition(leg.target_logical_block_id()));
                        }
                    }
                }
            }
            // Forward ExitControlMode to SubApp.
            cmd_queue.0.push(CommandEnvelope {
                command_id: *cmd_id,
                request: SimulationCommand::ExitControlMode(ExitControlModeRequest),
            });
        }
        AppCommand::Simulation(cmd) => {
            cmd_queue.0.push(CommandEnvelope {
                command_id: *cmd_id,
                request: cmd,
            });
        }
    }

    queue.in_flight = Some(entity);
}

/// Handles SpawnLayout commands: writes SpawnElement messages for all layout
/// elements and immediately completes the command.
fn handle_spawn_layout(
    mut messages: MessageReader<CommandEnvelope<SpawnLayoutRequest>>,
    mut spawn_tracks: MessageWriter<SpawnElement<Track>>,
    mut spawn_connections: MessageWriter<SpawnElement<Connection>>,
    mut spawn_markers: MessageWriter<SpawnElement<Marker>>,
    mut spawn_blocks: MessageWriter<SpawnElement<Block>>,
    mut spawn_trains: MessageWriter<SpawnElement<Train>>,
    mut response_writer: MessageWriter<CommandResponse>,
) {
    for envelope in messages.read() {
        let layout = &envelope.request.layout;
        for entry in &layout.tracks {
            spawn_tracks.write(SpawnElement::from_entry(entry));
        }
        for entry in &layout.connections {
            spawn_connections.write(SpawnElement::from_entry(entry));
        }
        for entry in &layout.markers {
            spawn_markers.write(SpawnElement::from_entry(entry));
        }
        for entry in &layout.blocks {
            spawn_blocks.write(SpawnElement::from_entry(entry));
        }
        for entry in &layout.trains {
            spawn_trains.write(SpawnElement::from_entry(entry));
        }
        response_writer.write(CommandResponse {
            command_id: envelope.command_id,
            result: Ok(()),
        });
    }
}
