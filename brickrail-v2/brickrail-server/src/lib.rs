use bevy::prelude::*;
use brickrail_common::layout::*;
use brickrail_common::lifecycle::*;
use brickrail_common::track::Track;

/// Handles entering control mode: loads the layout by spawning elements.
fn enter_control_mode(
    mut messages: MessageReader<EnterControlMode>,
    mut spawn_tracks: MessageWriter<SpawnElement<Track, ServerLayout>>,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for msg in messages.read() {
        for entry in &msg.layout.tracks {
            spawn_tracks.write(SpawnElement::from_entry(entry));
        }
        next_state.set(ServerState::Running);
    }
}

/// Handles exiting control mode: despawns all elements via relationship traversal.
fn exit_control_mode(
    mut messages: MessageReader<ExitControlMode>,
    layout_instance: Res<LayoutInstance<ServerLayout>>,
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
        app.add_plugins(LayoutInstancePlugin::<ServerLayout>::new());
        app.add_plugins(LifecyclePlugin::<Track, ServerLayout>::new());
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
