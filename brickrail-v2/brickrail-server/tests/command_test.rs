use bevy::prelude::*;
use brickrail_common::block::{Block, BlockData};
use brickrail_common::command::{
    CommandPlugin, CommandRegistry, CommandState, EnterControlModeRequest,
    PlaceTrainAtBlockRequest, SendTrainToBlockRequest, SimulationCommand, SubAppClientPlugin,
};
use brickrail_common::connection::Connection;
use brickrail_common::layout::{Layout, LayoutSubApp};
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::marker::Marker;
use brickrail_common::route::{RouteLeg, TrainLegs};
use brickrail_common::track::Track;
use brickrail_common::train::Train;
use brickrail_common::train_position::{TrainLegState, TrainPosition};
use brickrail_common::virtual_driver::VirtualDriver;

/// Build a client+simulation app with bidirectional extract bridge.
fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CommandPlugin);
    app.add_plugins(SubAppClientPlugin);
    app
}

/// Spawn layout elements into the SubApp.
fn spawn_layout(app: &mut App, layout: &Layout) {
    let world = app.sub_app_mut(LayoutSubApp).world_mut();
    for entry in &layout.tracks {
        world.write_message(SpawnElement::<Track>::from_entry(entry));
    }
    for entry in &layout.connections {
        world.write_message(SpawnElement::<Connection>::from_entry(entry));
    }
    for entry in &layout.markers {
        world.write_message(SpawnElement::<Marker>::from_entry(entry));
    }
    for entry in &layout.blocks {
        world.write_message(SpawnElement::<Block>::from_entry(entry));
    }
    for entry in &layout.trains {
        world.write_message(SpawnElement::<Train>::from_entry(entry));
    }
}

/// Two-block layout: [A: t0-t1] -- t2 -- [B: t3-t4]
fn two_block_layout() -> Layout {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    Layout {
        tracks: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t2, Default::default()),
            ElementEntry::new(t3, Default::default()),
            ElementEntry::new(t4, Default::default()),
        ],
        connections: vec![
            ElementEntry::new(t0.get_connection_to(t1).unwrap(), Default::default()),
            ElementEntry::new(t1.get_connection_to(t2).unwrap(), Default::default()),
            ElementEntry::new(t2.get_connection_to(t3).unwrap(), Default::default()),
            ElementEntry::new(t3.get_connection_to(t4).unwrap(), Default::default()),
        ],
        markers: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t3, Default::default()),
            ElementEntry::new(t4, Default::default()),
        ],
        blocks: vec![
            ElementEntry::new(
                BlockID::new(t0, t1),
                BlockData {
                    name: Some("A".to_string()),
                    section: vec![
                        t0.get_directed_to(Cardinal::E).unwrap(),
                        t1.get_directed_to(Cardinal::E).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
            ElementEntry::new(
                BlockID::new(t3, t4),
                BlockData {
                    name: Some("B".to_string()),
                    section: vec![
                        t3.get_directed_to(Cardinal::E).unwrap(),
                        t4.get_directed_to(Cardinal::E).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
        ],
        trains: vec![ElementEntry::new(TrainID(0), Default::default())],
    }
}

fn block_a() -> LogicalBlockID {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    LogicalBlockID {
        block: BlockID::new(t0, t1),
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    }
}

fn block_b() -> LogicalBlockID {
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    LogicalBlockID {
        block: BlockID::new(t3, t4),
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    }
}

#[test]
fn enter_control_mode() {
    let mut app = make_app();
    let layout = two_block_layout();

    let cmd_entity = CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::EnterControlMode(EnterControlModeRequest {
            layout: layout.clone(),
        }),
    );

    for _ in 0..3 {
        app.update();
    }

    // Command should be completed.
    let state = app.world().get::<CommandState>(cmd_entity).unwrap();
    assert!(
        matches!(state, CommandState::Completed),
        "expected Completed, got {:?}",
        state
    );

    // All layout elements should exist in the SubApp.
    {
        let sub_world = app.sub_app(LayoutSubApp).world();
        let track_registry = sub_world.resource::<Registry<Track>>();
        assert_eq!(track_registry.len(), layout.tracks.len());

        let block_registry = sub_world.resource::<Registry<Block>>();
        assert_eq!(block_registry.len(), layout.blocks.len());

        let train_registry = sub_world.resource::<Registry<Train>>();
        assert_eq!(train_registry.len(), layout.trains.len());
    }

    // VirtualDrivers should have been spawned (one per train).
    let sub_world = app.sub_app_mut(LayoutSubApp).world_mut();
    let driver_count = sub_world.query::<&VirtualDriver>().iter(sub_world).count();
    assert_eq!(driver_count, layout.trains.len());
}

#[test]
fn place_train_at_block() {
    let mut app = make_app();
    spawn_layout(&mut app, &two_block_layout());
    app.update(); // spawn layout elements

    // Issue PlaceTrainAtBlock command.
    let cmd_entity = CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::PlaceTrainAtBlock(PlaceTrainAtBlockRequest {
            train: TrainID(0),
            block: block_a(),
        }),
    );

    // Pipeline: dispatch → extract → SubApp processes → extract response → apply
    for _ in 0..3 {
        app.update();
    }

    // Command should be completed.
    let state = app.world().get::<CommandState>(cmd_entity).unwrap();
    assert!(
        matches!(state, CommandState::Completed),
        "expected Completed, got {:?}",
        state
    );

    // Train should have a position in the SubApp.
    let sub_world = app.sub_app(LayoutSubApp).world();
    let train_registry = sub_world.resource::<Registry<Train>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = sub_world.get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
}

#[test]
fn send_train_to_block() {
    let mut app = make_app();
    spawn_layout(&mut app, &two_block_layout());
    app.update();

    // Place train at block A first.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::PlaceTrainAtBlock(PlaceTrainAtBlockRequest {
            train: TrainID(0),
            block: block_a(),
        }),
    );
    for _ in 0..3 {
        app.update();
    }

    // Issue SendTrainToBlock command.
    let cmd_entity = CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::SendTrainToBlock(SendTrainToBlockRequest {
            train: TrainID(0),
            target_block: block_b(),
        }),
    );

    for _ in 0..3 {
        app.update();
    }

    // Command should be completed.
    let state = app.world().get::<CommandState>(cmd_entity).unwrap();
    assert!(
        matches!(state, CommandState::Completed),
        "expected Completed, got {:?}",
        state
    );

    // Train should have route legs in the SubApp.
    let sub_world = app.sub_app(LayoutSubApp).world();
    let train_registry = sub_world.resource::<Registry<Train>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let legs = sub_world.get::<TrainLegs>(train_entity).unwrap();
    // Should have: idle (current) + route + trailing idle
    assert!(
        legs.collection().len() >= 2,
        "expected at least 2 legs, got {}",
        legs.collection().len()
    );
}

#[test]
fn send_train_to_block_end_to_end() {
    let mut app = make_app();
    let layout = two_block_layout();

    // Enter control mode — spawns layout + VirtualDrivers.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::EnterControlMode(EnterControlModeRequest { layout }),
    );
    for _ in 0..3 {
        app.update();
    }

    // Boost VirtualDriver speed for fast test traversal.
    let sub_world = app.sub_app_mut(LayoutSubApp).world_mut();
    for mut driver in sub_world.query::<&mut VirtualDriver>().iter_mut(sub_world) {
        driver.speed = 1_000_000.0;
    }

    // Place train at block A.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::PlaceTrainAtBlock(PlaceTrainAtBlockRequest {
            train: TrainID(0),
            block: block_a(),
        }),
    );
    for _ in 0..3 {
        app.update();
    }

    // Send train to block B.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::SendTrainToBlock(SendTrainToBlockRequest {
            train: TrainID(0),
            target_block: block_b(),
        }),
    );

    // Run many frames for the full pipeline.
    for _ in 0..30 {
        app.update();
    }

    // Train should have arrived at block B (trailing idle).
    let sub_world = app.sub_app(LayoutSubApp).world();
    let train_registry = sub_world.resource::<Registry<Train>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = sub_world.get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);

    let legs = sub_world.get::<TrainLegs>(train_entity).unwrap();
    assert_eq!(
        legs.collection().len(),
        1,
        "only trailing idle should remain"
    );
    let current_leg = sub_world
        .get::<RouteLeg>(*legs.collection().first().unwrap())
        .unwrap();
    assert_eq!(current_leg.target_block.block_id, block_b().block);
}
