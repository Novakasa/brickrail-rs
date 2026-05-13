use std::collections::VecDeque;

use bevy::prelude::*;

use crate::block::Block;
use crate::connection::Connection;
use crate::layout::Layout;
use crate::lifecycle::SpawnElement;
use crate::marker::Marker;
use crate::track::Track;
use crate::train::Train;

use super::{
    CommandEnvelope, CommandId, CommandRegistry, CommandResponse, CommandState, SimulationCommand,
    SubAppCommandInputQueue,
};

/// Top-level command enum for the client side.
/// Covers both client-local commands and simulation commands forwarded to the SubApp.
#[derive(Component, Clone, Debug)]
pub enum AppCommand {
    /// Spawn layout elements in the main world (for rendering/editing).
    SpawnLayout(Layout),
    /// Forward a simulation command to the SubApp.
    Simulation(SimulationCommand),
}

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
