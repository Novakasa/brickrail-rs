use bevy::prelude::*;
use brickrail_common::layout::*;
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::block::Block;
use brickrail_common::connection::{Connection, ConnectionGraph};
use brickrail_common::marker::Marker;
use brickrail_common::track::Track;
use brickrail_common::train::Train;
use brickrail_server::ServerPlugin;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ServerPlugin);
    app
}

fn test_layout() -> Layout {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);

    Layout {
        tracks: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t2, Default::default()),
        ],
        connections: vec![
            ElementEntry::new(t0.get_connection_to(t1).unwrap(), Default::default()),
            ElementEntry::new(t1.get_connection_to(t2).unwrap(), Default::default()),
        ],
        markers: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t2, Default::default()),
        ],
        blocks: vec![
            ElementEntry::new(
                BlockID::new(t0, t2),
                brickrail_common::block::BlockData {
                    name: Some("Main".to_string()),
                    section: vec![
                        t0.get_directed_to(Cardinal::E).unwrap(),
                        t1.get_directed_to(Cardinal::E).unwrap(),
                        t2.get_directed_to(Cardinal::W).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
        ],
        trains: vec![
            ElementEntry::new(TrainID(0), Default::default()),
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

    // Connections from layout should be spawned
    let conn_registry = app.world().resource::<Registry<Connection, ServerLayout>>();
    assert_eq!(conn_registry.len(), 2);

    // Connection graph should reflect the topology
    let graph = app.world().resource::<ConnectionGraph<ServerLayout>>();
    assert_eq!(graph.graph.node_count(), 3);
    assert_eq!(graph.graph.edge_count(), 2);
    // Middle track should have 2 connections
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    assert_eq!(graph.connections_from(t1).len(), 2);

    // Markers should be spawned
    let marker_registry = app.world().resource::<Registry<Marker, ServerLayout>>();
    assert_eq!(marker_registry.len(), 2);

    // Block should be spawned with correct data
    let block_registry = app.world().resource::<Registry<Block, ServerLayout>>();
    assert_eq!(block_registry.len(), 1);
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let block_entity = block_registry.get(&BlockID::new(t0, t2)).unwrap();
    let block_data = app.world().get::<ElementData<Block>>(block_entity).unwrap();
    assert_eq!(block_data.section.len(), 3);
    assert_eq!(block_data.name, Some("Main".to_string()));

    // Train should be spawned
    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    assert_eq!(train_registry.len(), 1);

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
    app.update();

    // Exit
    app.world_mut().write_message(ExitControlMode);
    app.update();

    // Registries should be empty
    let registry = app.world().resource::<Registry<Track, ServerLayout>>();
    assert_eq!(registry.len(), 0);
    let conn_registry = app.world().resource::<Registry<Connection, ServerLayout>>();
    assert_eq!(conn_registry.len(), 0);
    let graph = app.world().resource::<ConnectionGraph<ServerLayout>>();
    assert_eq!(graph.graph.edge_count(), 0);
    let marker_registry = app.world().resource::<Registry<Marker, ServerLayout>>();
    assert_eq!(marker_registry.len(), 0);
    let block_registry = app.world().resource::<Registry<Block, ServerLayout>>();
    assert_eq!(block_registry.len(), 0);
    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    assert_eq!(train_registry.len(), 0);

    // State should be back to Idle
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
