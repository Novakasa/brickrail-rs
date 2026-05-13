use bevy::ecs::relationship::RelationshipTarget;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use petgraph::algo::astar;

use crate::block::{Block, BlockData};
use crate::command::{
    CommandEnvelope, CommandResponse, EnterControlModeRequest, PlaceTrainAtBlockRequest,
    SendTrainToBlockRequest,
};
use crate::connection::Connection;
use crate::driver::{DriverLeg, DriverMarkerHit, QueueDriverLeg};
use crate::layout_primitives::{BlockDirection, BlockID, LogicalBlockID, TrackID};
use crate::lifecycle::{ElementData, ElementId, Registry, SpawnElement};
use crate::logical_graph::LogicalGraph;
use crate::marker::{Marker, MarkerData};
use crate::route::{AppendLegs, LegOf, Locked, RouteLeg, TrainLegs};
use crate::simulation_event::SimulationEvent;
use crate::track::Track;
use crate::train::Train;
use crate::train_position::{AdvanceLeg, TrainLegState, TrainMarkerHit, TrainPosition};
use crate::virtual_driver::VirtualDriver;

/// System sets for ordering simulation systems within `Update`.
/// State mutation runs first (processing messages), then logic reacts to the new state.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    /// Systems that mutate state in response to messages (e.g. handle_train_marker_hit).
    StateMutation,
    /// Systems that read state and emit new messages (e.g. advance_leg_logic).
    Logic,
}

/// Simulation state plugin. Shared by both server and client.
/// Registers `SimulationEvent` message and fan-out, plus mutation systems.
/// Does NOT include logic systems — those go in `SimulationLogicPlugin`.
pub struct SimulationStatePlugin;

impl Plugin for SimulationStatePlugin {
    fn build(&self, app: &mut App) {
        use crate::route::RouteStatePlugin;
        use crate::train_position::TrainPositionStatePlugin;

        app.configure_sets(
            Update,
            SimulationSet::Logic.after(SimulationSet::StateMutation),
        );
        app.add_message::<SimulationEvent>();
        app.add_systems(
            Update,
            fan_out_simulation_events
                .run_if(on_message::<SimulationEvent>)
                .before(SimulationSet::StateMutation),
        );
        app.add_plugins(RouteStatePlugin);
        app.add_plugins(TrainPositionStatePlugin);
    }
}

/// Fan-out: reads `SimulationEvent` messages and dispatches them to individual
/// message types for the modular handlers to consume.
fn fan_out_simulation_events(
    mut event_reader: MessageReader<SimulationEvent>,
    mut append_writer: MessageWriter<AppendLegs>,
    mut marker_hit_writer: MessageWriter<TrainMarkerHit>,
    mut advance_writer: MessageWriter<AdvanceLeg>,
) {
    for event in event_reader.read() {
        match event {
            SimulationEvent::AppendLegs(e) => {
                append_writer.write(e.clone());
            }
            SimulationEvent::TrainMarkerHit(e) => {
                marker_hit_writer.write(e.clone());
            }
            SimulationEvent::AdvanceLeg(e) => {
                advance_writer.write(e.clone());
            }
        }
    }
}

/// Top-level simulation plugin bundling all communication-agnostic domain logic.
/// Does NOT include transport or command handling — those are added separately
/// by the transport layer (e.g. `SubAppClientPlugin`).
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::layout::LayoutAppPlugin);
        app.add_plugins(SimulationLogicPlugin);
        app.add_plugins(bevy::time::TimePlugin);
        app.add_plugins(crate::virtual_driver::VirtualDriverPlugin);
    }
}

/// Simulation logic plugin. Server-side only.
/// Contains logic systems that react to state and emit `SimulationEvent` messages:
/// leg advancement, driver dispatch, and driver↔simulation translation.
/// Requires `SimulationStatePlugin`.
pub struct SimulationLogicPlugin;

impl Plugin for SimulationLogicPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<QueueDriverLeg>();
        app.add_message::<DriverMarkerHit>();
        app.add_observer(dispatch_locked_leg);
        app.add_systems(
            Update,
            (
                advance_leg_logic,
                translate_driver_marker_hit.run_if(on_message::<DriverMarkerHit>),
            )
                .in_set(SimulationSet::Logic),
        );
    }
}

/// Simulation logic: when a train has fully entered its target block and has
/// a next leg queued, advance to that next leg.
fn advance_leg_logic(
    query: Query<(&TrainPosition, &TrainLegs, &ElementId<Train>)>,
    mut event_writer: MessageWriter<SimulationEvent>,
) {
    for (position, legs, element_id) in &query {
        if position.leg_state == TrainLegState::EnteredTarget && legs.collection().len() > 1 {
            event_writer.write(SimulationEvent::AdvanceLeg(AdvanceLeg::new(element_id.0)));
        }
    }
}

/// Observer: when a leg gets `Locked`, dispatch it to the driver.
/// Converts the `RouteLeg` to a `DriverLeg` and sends `QueueDriverLeg`.
fn dispatch_locked_leg(
    trigger: On<Add, Locked>,
    leg_query: Query<(&RouteLeg, &LegOf)>,
    train_id_query: Query<&ElementId<Train>>,
    mut queue_writer: MessageWriter<QueueDriverLeg>,
) {
    let leg_entity = trigger.event().entity;
    let Ok((leg, leg_of)) = leg_query.get(leg_entity) else {
        return;
    };
    let Ok(train_element_id) = train_id_query.get(leg_of.0) else {
        return;
    };
    queue_writer.write(QueueDriverLeg::new(
        train_element_id.0,
        DriverLeg::from(leg),
    ));
}

/// Translation: converts `DriverMarkerHit` from the driver layer into
/// `TrainMarkerHit` for the simulation state layer.
fn translate_driver_marker_hit(
    mut driver_hits: MessageReader<DriverMarkerHit>,
    mut event_writer: MessageWriter<SimulationEvent>,
) {
    for hit in driver_hits.read() {
        event_writer.write(SimulationEvent::TrainMarkerHit(TrainMarkerHit::new(
            hit.train,
        )));
    }
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

/// Build block data map from ECS registries.
fn build_block_data_map(
    registry: &Registry<Block>,
    query: &Query<&ElementData<Block>>,
) -> HashMap<BlockID, BlockData> {
    let mut map = HashMap::new();
    for (id, &entity) in registry.iter() {
        if let Ok(data) = query.get(entity) {
            map.insert(*id, data.0.clone());
        }
    }
    map
}

/// Build marker data map from ECS registries.
fn build_marker_data_map(
    registry: &Registry<Marker>,
    query: &Query<&ElementData<Marker>>,
) -> HashMap<TrackID, MarkerData> {
    let mut map = HashMap::new();
    for (id, &entity) in registry.iter() {
        if let Ok(data) = query.get(entity) {
            map.insert(*id, data.0.clone());
        }
    }
    map
}

/// Command handler: enter control mode by spawning layout elements + VirtualDrivers.
pub fn handle_enter_control_mode(
    mut messages: MessageReader<CommandEnvelope<EnterControlModeRequest>>,
    mut spawn_tracks: MessageWriter<SpawnElement<Track>>,
    mut spawn_connections: MessageWriter<SpawnElement<Connection>>,
    mut spawn_markers: MessageWriter<SpawnElement<Marker>>,
    mut spawn_blocks: MessageWriter<SpawnElement<Block>>,
    mut spawn_trains: MessageWriter<SpawnElement<Train>>,
    mut commands: Commands,
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
        // Spawn a VirtualDriver per train.
        for entry in &layout.trains {
            commands.spawn(VirtualDriver::new(entry.id, 1.0));
        }
        response_writer.write(CommandResponse {
            command_id: envelope.command_id,
            result: Ok(()),
        });
    }
}

/// Command handler: place a train at a block by creating an idle leg.
pub fn handle_place_train_at_block(
    mut messages: MessageReader<CommandEnvelope<PlaceTrainAtBlockRequest>>,
    block_registry: Res<Registry<Block>>,
    block_data_query: Query<&ElementData<Block>>,
    marker_registry: Res<Registry<Marker>>,
    marker_data_query: Query<&ElementData<Marker>>,
    mut event_writer: MessageWriter<SimulationEvent>,
    mut response_writer: MessageWriter<CommandResponse>,
) {
    let block_data_map = build_block_data_map(&block_registry, &block_data_query);
    let marker_data_map = build_marker_data_map(&marker_registry, &marker_data_query);

    for envelope in messages.read() {
        let req = &envelope.request;
        let result = match RouteLeg::idle(req.block, &block_data_map, &marker_data_map) {
            Some(idle_leg) => {
                event_writer.write(SimulationEvent::AppendLegs(AppendLegs::new(
                    req.train,
                    vec![idle_leg],
                )));
                Ok(())
            }
            None => Err(format!("block {:?} not found", req.block)),
        };
        response_writer.write(CommandResponse {
            command_id: envelope.command_id,
            result,
        });
    }
}

/// Command handler: send a train from its current block to a target block.
/// Pathfinds, builds route legs + trailing idle, and appends them.
pub fn handle_send_train_to_block(
    mut messages: MessageReader<CommandEnvelope<SendTrainToBlockRequest>>,
    logical_graph: Res<LogicalGraph>,
    block_registry: Res<Registry<Block>>,
    block_data_query: Query<&ElementData<Block>>,
    marker_registry: Res<Registry<Marker>>,
    marker_data_query: Query<&ElementData<Marker>>,
    train_registry: Res<Registry<Train>>,
    train_query: Query<(&TrainPosition, &TrainLegs)>,
    leg_query: Query<&RouteLeg>,
    mut event_writer: MessageWriter<SimulationEvent>,
    mut response_writer: MessageWriter<CommandResponse>,
) {
    let block_data_map = build_block_data_map(&block_registry, &block_data_query);
    let marker_data_map = build_marker_data_map(&marker_registry, &marker_data_query);

    for envelope in messages.read() {
        let req = &envelope.request;
        let result = (|| -> Result<(), String> {
            // Resolve train entity.
            let train_entity = train_registry
                .get(&req.train)
                .ok_or_else(|| format!("train {:?} not found", req.train))?;

            // Get train's current block from its current leg.
            let (_, legs) = train_query
                .get(train_entity)
                .map_err(|_| "train has no position/legs".to_string())?;
            let current_leg_entity = *legs
                .collection()
                .first()
                .ok_or("train has no current leg")?;
            let current_leg = leg_query
                .get(current_leg_entity)
                .map_err(|_| "current leg entity missing")?;

            // Determine start block from current leg's target block.
            let start_block_id = current_leg.target_block.block_id;
            let start = LogicalBlockID {
                block: start_block_id,
                direction: BlockDirection::Aligned,
                facing: current_leg.facing,
            };

            // Pathfind from start to target.
            let start_data = block_data_map
                .get(&start.block)
                .ok_or_else(|| format!("start block {:?} not found", start.block))?;
            let target_data = block_data_map
                .get(&req.target_block.block)
                .ok_or_else(|| format!("target block {:?} not found", req.target_block.block))?;
            let start_track = start_data.enter_logical_track(start.direction, start.facing);
            let target_track = target_data
                .enter_logical_track(req.target_block.direction, req.target_block.facing);

            let (_cost, path) = astar(
                &logical_graph.graph,
                start_track,
                |n| n == target_track,
                |_| 1u32,
                |_| 0u32,
            )
            .ok_or("no path found")?;

            let mut route_legs =
                RouteLeg::build_from_path(&path, &block_data_map, &marker_data_map)
                    .ok_or("failed to build route legs")?;

            // Add trailing idle at target.
            let trailing_idle = RouteLeg::idle(req.target_block, &block_data_map, &marker_data_map)
                .ok_or("failed to build trailing idle")?;
            route_legs.push(trailing_idle);

            event_writer.write(SimulationEvent::AppendLegs(AppendLegs::new(
                req.train, route_legs,
            )));
            Ok(())
        })();

        response_writer.write(CommandResponse {
            command_id: envelope.command_id,
            result,
        });
    }
}
