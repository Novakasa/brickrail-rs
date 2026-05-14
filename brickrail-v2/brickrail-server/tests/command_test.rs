use bevy::prelude::*;
use brickrail_common::block::{Block, BlockData};
use brickrail_common::command::{
    AppCommand, AppCommandPlugin, AppCommandQueue, CommandPlugin, CommandRegistry, CommandState,
    EnterControlModeRequest, SendTrainToBlockRequest, SimulationCommand, SubAppClientPlugin,
};
use brickrail_common::layout::{Layout, LayoutAppPlugin, LayoutSubApp};
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
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
            train_positions: vec![],
        }),
    );

    for _ in 0..5 {
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
fn enter_control_mode_with_train_placement() {
    let mut app = make_app();
    let layout = two_block_layout();

    // Enter control mode with train placed at block A.
    let cmd_entity = CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::EnterControlMode(EnterControlModeRequest {
            layout,
            train_positions: vec![(TrainID(0), block_a())],
        }),
    );

    // Pipeline: dispatch → enter → spawn → place trains → running
    for _ in 0..5 {
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

    // Enter control mode with train placed at block A.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::EnterControlMode(EnterControlModeRequest {
            layout: two_block_layout(),
            train_positions: vec![(TrainID(0), block_a())],
        }),
    );
    for _ in 0..10 {
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

    // Enter control mode with train placed at block A.
    CommandRegistry::issue_world(
        app.world_mut(),
        SimulationCommand::EnterControlMode(EnterControlModeRequest {
            layout,
            train_positions: vec![(TrainID(0), block_a())],
        }),
    );
    for _ in 0..5 {
        app.update();
    }

    // Boost VirtualDriver speed for fast test traversal.
    let sub_world = app.sub_app_mut(LayoutSubApp).world_mut();
    for mut driver in sub_world.query::<&mut VirtualDriver>().iter_mut(sub_world) {
        driver.speed = 1_000_000.0;
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

/// Build a full client-level app with AppCommandQueue and main-world state mirror.
fn make_client_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CommandPlugin);
    app.add_plugins(AppCommandPlugin);
    app.add_plugins(LayoutAppPlugin);
    app.add_plugins(SubAppClientPlugin);
    app
}

#[test]
fn exit_and_reenter_preserves_position() {
    let mut app = make_client_app();
    let layout = two_block_layout();

    // Spawn layout in main world (for state mirror).
    AppCommandQueue::push_world(app.world_mut(), AppCommand::SpawnLayout(layout.clone()));

    // Set initial train position and enter control mode.
    AppCommandQueue::push_world(
        app.world_mut(),
        AppCommand::SetTrainPosition(TrainID(0), block_a()),
    );
    AppCommandQueue::push_world(
        app.world_mut(),
        AppCommand::EnterControlMode(layout.clone()),
    );

    // Send train to block B via simulation command.
    AppCommandQueue::push_world(
        app.world_mut(),
        AppCommand::Simulation(SimulationCommand::SendTrainToBlock(
            SendTrainToBlockRequest {
                train: TrainID(0),
                target_block: block_b(),
            },
        )),
    );

    // Run enough frames for enter + place + send to complete.
    for _ in 0..10 {
        app.update();
    }

    // Boost VirtualDriver speed for fast traversal.
    let sub_world = app.sub_app_mut(LayoutSubApp).world_mut();
    for mut driver in sub_world.query::<&mut VirtualDriver>().iter_mut(sub_world) {
        driver.speed = 1_000_000.0;
    }

    // Run many frames for the train to arrive.
    for _ in 0..30 {
        app.update();
    }

    // Verify train arrived at block B in SubApp.
    {
        let sub_world = app.sub_app(LayoutSubApp).world();
        let train_registry = sub_world.resource::<Registry<Train>>();
        let train_entity = train_registry.get(&TrainID(0)).unwrap();
        let position = sub_world.get::<TrainPosition>(train_entity).unwrap();
        assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
    }

    // Exit control mode via AppCommand — should extract TrainBlockPosition.
    let exit_cmd = AppCommandQueue::push_world(app.world_mut(), AppCommand::ExitControlMode);
    for _ in 0..5 {
        app.update();
    }
    let state = app.world().get::<CommandState>(exit_cmd).unwrap();
    assert!(
        matches!(state, CommandState::Completed),
        "exit expected Completed, got {:?}",
        state
    );

    // Verify TrainBlockPosition was cached on main-world train entity.
    {
        use brickrail_common::command::TrainBlockPosition;
        let main_world = app.world();
        let train_registry = main_world.resource::<Registry<Train>>();
        let train_entity = train_registry.get(&TrainID(0)).unwrap();
        let cached = main_world.get::<TrainBlockPosition>(train_entity).unwrap();
        assert_eq!(
            cached.0.block,
            block_b().block,
            "cached position should be block B"
        );
    }

    // Re-enter control mode — should auto-place train at cached position.
    let enter_cmd =
        AppCommandQueue::push_world(app.world_mut(), AppCommand::EnterControlMode(layout));
    for _ in 0..10 {
        app.update();
    }
    let state = app.world().get::<CommandState>(enter_cmd).unwrap();
    assert!(
        matches!(state, CommandState::Completed),
        "re-enter expected Completed, got {:?}",
        state
    );

    // Run a few more frames for the auto-placed PlaceTrainAtBlock to complete.
    for _ in 0..5 {
        app.update();
    }

    // Verify train is placed at block B in SubApp.
    let sub_world = app.sub_app(LayoutSubApp).world();
    let train_registry = sub_world.resource::<Registry<Train>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = sub_world.get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);

    let legs = sub_world.get::<TrainLegs>(train_entity).unwrap();
    let current_leg = sub_world
        .get::<RouteLeg>(*legs.collection().first().unwrap())
        .unwrap();
    assert_eq!(
        current_leg.target_block.block_id,
        block_b().block,
        "train should be at block B after round-trip"
    );
}
