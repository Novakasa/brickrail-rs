use bevy::prelude::*;
use brickrail_common::layout::*;
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::track::Track;
use brickrail_server::ServerPlugin;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ServerPlugin);
    app
}

fn test_layout() -> Layout {
    Layout {
        tracks: vec![
            Track::new(TrackID::new(CellID::new(0, 0, 0), Orientation::EW)),
            Track::new(TrackID::new(CellID::new(1, 0, 0), Orientation::EW)),
            Track::new(TrackID::new(CellID::new(2, 0, 0), Orientation::NE)),
        ],
    }
}

#[test]
fn enter_control_mode_spawns_layout() {
    let mut app = make_app();
    let layout = test_layout();

    app.world_mut()
        .write_message(EnterControlMode { layout });
    app.update();

    // All tracks should be spawned and registered
    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
    assert_eq!(registry.len(), 3);

    // State transition via NextState takes effect next frame
    app.update();
    let state = app.world().resource::<State<ServerState>>();
    assert_eq!(*state.get(), ServerState::Running);
}

#[test]
fn exit_control_mode_cleans_up() {
    let mut app = make_app();
    let layout = test_layout();

    // Enter
    app.world_mut()
        .write_message(EnterControlMode { layout });
    app.update();

    // State transition happens next frame
    app.update();

    // Exit
    app.world_mut().write_message(ExitControlMode);
    app.update();

    // Registry should be empty
    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
    assert_eq!(registry.len(), 0);

    // State should be back to Idle (may need another update for state transition)
    app.update();
    let state = app.world().resource::<State<ServerState>>();
    assert_eq!(*state.get(), ServerState::Idle);
}

#[test]
fn round_trip_enter_exit_enter() {
    let mut app = make_app();
    let layout = test_layout();

    // First enter
    app.world_mut()
        .write_message(EnterControlMode {
            layout: layout.clone(),
        });
    app.update();
    assert_eq!(
        app.world()
            .resource::<Registry<Track, ServerLayout>>()
            .len(),
        3
    );

    app.update();

    // Exit
    app.world_mut().write_message(ExitControlMode);
    app.update();
    assert_eq!(
        app.world()
            .resource::<Registry<Track, ServerLayout>>()
            .len(),
        0
    );

    app.update();

    // Second enter
    app.world_mut()
        .write_message(EnterControlMode { layout });
    app.update();
    assert_eq!(
        app.world()
            .resource::<Registry<Track, ServerLayout>>()
            .len(),
        3
    );
}
