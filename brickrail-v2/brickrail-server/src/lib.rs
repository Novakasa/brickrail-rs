use bevy::app::{Main, MainSchedulePlugin, SubApp};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use brickrail_common::block::Block;
use brickrail_common::connection::Connection;
use brickrail_common::layout::*;
use brickrail_common::lifecycle::{
    despawn_all_elements, RegisteredEntities, SpawnElement,
};
use brickrail_common::marker::Marker;
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

/// Handles exiting control mode: despawns all elements.
fn exit_control_mode(
    mut messages: MessageReader<ExitControlMode>,
    registries: Query<&RegisteredEntities>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for _msg in messages.read() {
        despawn_all_elements(&registries, &mut commands);
        next_state.set(ServerState::Idle);
    }
}

/// Server app plugin. Creates a layout SubApp with the control mode state machine.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        let mut sub_app = SubApp::new();
        sub_app.update_schedule = Some(Main.intern());

        // Bootstrap the SubApp with standard Bevy schedules and message plumbing.
        sub_app.add_plugins(MainSchedulePlugin);
        sub_app.add_systems(
            First,
            bevy::ecs::message::message_update_system
                .in_set(bevy::ecs::message::MessageUpdateSystems)
                .run_if(bevy::ecs::message::message_update_condition),
        );

        sub_app.add_plugins(LayoutAppPlugin);
        sub_app.add_plugins(brickrail_common::simulation::SimulationLogicPlugin);
        sub_app.add_plugins(brickrail_common::simulation::SimulationCollectorPlugin);
        sub_app.add_plugins(StatesPlugin);
        sub_app.init_state::<ServerState>();
        sub_app.add_message::<EnterControlMode>();
        sub_app.add_message::<ExitControlMode>();
        sub_app.add_systems(
            Update,
            (
                enter_control_mode.run_if(on_message::<EnterControlMode>),
                exit_control_mode.run_if(on_message::<ExitControlMode>),
            ),
        );
        app.insert_sub_app(LayoutSubApp, sub_app);
    }
}
