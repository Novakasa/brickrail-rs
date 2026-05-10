use bevy::prelude::*;
use brickrail_common::block::Block;
use brickrail_common::connection::Connection;
use brickrail_common::layout::*;
use brickrail_common::lifecycle::*;
use brickrail_common::logical_graph::LogicalGraphPlugin;
use brickrail_common::marker::Marker;
use brickrail_common::simulation::SimulationStatePlugin;
use brickrail_common::track::Track;
use brickrail_common::train::Train;

/// Handles entering control mode: loads the layout by spawning elements.
fn enter_control_mode(
    mut messages: MessageReader<EnterControlMode>,
    mut spawn_tracks: MessageWriter<SpawnElement<Track>>,
    mut spawn_connections: MessageWriter<SpawnElement<Connection>>,
    mut spawn_markers: MessageWriter<SpawnElement<Marker>>,
    mut spawn_blocks: MessageWriter<SpawnElement<Block>>,
    mut spawn_trains: MessageWriter<SpawnElement<Train>>,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for msg in messages.read() {
        for entry in &msg.layout.tracks {
            spawn_tracks.write(SpawnElement::from_entry(entry));
        }
        for entry in &msg.layout.connections {
            spawn_connections.write(SpawnElement::from_entry(entry));
        }
        for entry in &msg.layout.markers {
            spawn_markers.write(SpawnElement::from_entry(entry));
        }
        for entry in &msg.layout.blocks {
            spawn_blocks.write(SpawnElement::from_entry(entry));
        }
        for entry in &msg.layout.trains {
            spawn_trains.write(SpawnElement::from_entry(entry));
        }
        next_state.set(ServerState::Running);
    }
}

/// Handles exiting control mode: despawns all elements via relationship traversal.
fn exit_control_mode(
    mut messages: MessageReader<ExitControlMode>,
    layout_instance: Res<LayoutInstance>,
    registries: Query<&RegisteredEntities>,
    layout_registries: Query<&Registries>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for _msg in messages.read() {
        despawn_all_in_layout(
            layout_instance.entity,
            &registries,
            &layout_registries,
            &mut commands,
        );
        next_state.set(ServerState::Idle);
    }
}

/// Server app plugin. Manages the control mode state machine.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ServerState>();
        app.add_plugins(LayoutInstancePlugin);
        app.add_plugins(ElementPlugin::<Track>::new());
        app.add_plugins(ElementPlugin::<Connection>::new());
        app.add_plugins(ElementPlugin::<Marker>::new());
        app.add_plugins(ElementPlugin::<Block>::new());
        app.add_plugins(ElementPlugin::<Train>::new());
        app.add_plugins(LogicalGraphPlugin);
        app.add_plugins(SimulationStatePlugin);
        app.add_message::<EnterControlMode>();
        app.add_message::<ExitControlMode>();
        app.add_systems(
            Update,
            (
                enter_control_mode.run_if(on_message::<EnterControlMode>),
                exit_control_mode.run_if(on_message::<ExitControlMode>),
            ),
        );
    }
}
