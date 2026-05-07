use bevy::prelude::*;
use brickrail_common::layout::*;
use brickrail_common::lifecycle::*;
use brickrail_common::track::Track;

/// Handles entering control mode: loads the layout by spawning elements.
fn enter_control_mode(
    mut messages: MessageReader<EnterControlMode>,
    mut spawn_tracks: MessageWriter<SpawnElement<Track>>,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for msg in messages.read() {
        for track in &msg.layout.tracks {
            spawn_tracks.write(SpawnElement(track.clone()));
        }
        next_state.set(ServerState::Running);
    }
}

/// Handles exiting control mode: despawns all elements.
fn exit_control_mode(
    mut messages: MessageReader<ExitControlMode>,
    registry: Res<Registry<Track>>,
    mut despawn_tracks: MessageWriter<DespawnElement<Track>>,
    mut next_state: ResMut<NextState<ServerState>>,
) {
    for _msg in messages.read() {
        for (id, _entity) in registry.iter() {
            despawn_tracks.write(DespawnElement::new(id.clone()));
        }
        next_state.set(ServerState::Idle);
    }
}

/// Server app plugin. Manages the control mode state machine.
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ServerState>();
        app.add_plugins(LifecyclePlugin::<Track>::new());
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
