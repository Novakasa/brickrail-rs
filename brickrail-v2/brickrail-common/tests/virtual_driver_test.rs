use bevy::ecs::relationship::RelationshipTarget;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use brickrail_common::block::{Block, BlockData};
use brickrail_common::connection::Connection;
use brickrail_common::layout::LayoutAppPlugin;
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::logical_graph::LogicalGraph;
use brickrail_common::marker::{Marker, MarkerData};
use brickrail_common::route::{AppendLegs, RouteLeg, TrainLegs};
use brickrail_common::simulation::SimulationLogicPlugin;
use brickrail_common::track::Track;
use brickrail_common::train::Train;
use brickrail_common::train_position::{TrainLegState, TrainPosition};
use brickrail_common::virtual_driver::{VirtualDriver, VirtualDriverPlugin};
use petgraph::algo::astar;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutAppPlugin);
    app.add_plugins(SimulationLogicPlugin);
    app.add_plugins(VirtualDriverPlugin);
    app
}

/// Spawn a two-block layout: [A: t0-t1] -- t2 -- [B: t3-t4]
fn spawn_two_block_layout(app: &mut App) {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    for t in [t0, t1, t2, t3, t4] {
        app.world_mut()
            .write_message(SpawnElement::<Track>::new(t, Default::default()));
    }
    for (a, b) in [(t0, t1), (t1, t2), (t2, t3), (t3, t4)] {
        app.world_mut()
            .write_message(SpawnElement::<Connection>::new(
                a.get_connection_to(b).unwrap(),
                Default::default(),
            ));
    }
    for t in [t0, t1, t3, t4] {
        app.world_mut()
            .write_message(SpawnElement::<Marker>::new(t, Default::default()));
    }
    app.world_mut().write_message(SpawnElement::<Block>::new(
        BlockID::new(t0, t1),
        BlockData {
            name: Some("A".to_string()),
            section: vec![
                t0.get_directed_to(Cardinal::E).unwrap(),
                t1.get_directed_to(Cardinal::E).unwrap(),
            ],
            ..Default::default()
        },
    ));
    app.world_mut().write_message(SpawnElement::<Block>::new(
        BlockID::new(t3, t4),
        BlockData {
            name: Some("B".to_string()),
            section: vec![
                t3.get_directed_to(Cardinal::E).unwrap(),
                t4.get_directed_to(Cardinal::E).unwrap(),
            ],
            ..Default::default()
        },
    ));
    app.world_mut()
        .write_message(SpawnElement::<Train>::new(TrainID(0), Default::default()));

    app.update();
}

fn extract_block_data_map(app: &App) -> HashMap<BlockID, BlockData> {
    let block_registry = app.world().resource::<Registry<Block>>();
    let mut map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = app.world().get::<ElementData<Block>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn extract_marker_data_map(app: &App) -> HashMap<TrackID, MarkerData> {
    let marker_registry = app.world().resource::<Registry<Marker>>();
    let mut map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = app.world().get::<ElementData<Marker>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn build_route(
    start: LogicalBlockID,
    target: LogicalBlockID,
    logical_graph: &LogicalGraph,
    block_data_map: &HashMap<BlockID, BlockData>,
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Option<Vec<RouteLeg>> {
    let start_data = block_data_map.get(&start.block)?;
    let target_data = block_data_map.get(&target.block)?;
    let start_track = start_data.enter_logical_track(start.direction, start.facing);
    let target_track = target_data.enter_logical_track(target.direction, target.facing);

    let (_cost, path) = astar(
        &logical_graph.graph,
        start_track,
        |n| n == target_track,
        |_| 1u32,
        |_| 0u32,
    )?;

    RouteLeg::build_from_path(&path, block_data_map, marker_data_map)
}

/// End-to-end test: virtual driver advances a train from block A to block B.
/// Uses a very high speed so markers are crossed within a single tick.
#[test]
fn virtual_driver_advances_through_route() {
    let mut app = make_app();
    spawn_two_block_layout(&mut app);

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);
    let logical_graph = app.world().resource::<LogicalGraph>();

    let logical_a = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let logical_b = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    // Build idle leg at A, route A→B, trailing idle at B
    let idle = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map).unwrap();
    let mut route_legs = build_route(
        logical_a,
        logical_b,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .unwrap();
    route_legs.push(RouteLeg::idle(logical_b, &block_data_map, &marker_data_map).unwrap());

    // Spawn VirtualDriver as a separate entity
    app.world_mut()
        .spawn(VirtualDriver::new(TrainID(0), 1_000_000.0));

    // Place train with idle leg
    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), vec![idle]));
    app.update();

    // Train should be idle at block A
    let train_entity = app
        .world()
        .resource::<Registry<Train>>()
        .get(&TrainID(0))
        .unwrap();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);

    // Append route + trailing idle
    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), route_legs));
    app.update();

    // Run enough updates for the full pipeline:
    // AppendLegs → dispatch → QueueDriverLeg → driver ticks → DriverMarkerHit → TrainMarkerHit → AdvanceLeg
    for _ in 0..20 {
        app.update();
    }

    // Train should have arrived at block B (trailing idle)
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
    assert_eq!(position.marker_index, 0);

    // Current leg should be the trailing idle at block B
    let legs = app.world().get::<TrainLegs>(train_entity).unwrap();
    assert_eq!(
        legs.collection().len(),
        1,
        "only trailing idle should remain"
    );
    let current_leg_entity = *legs.collection().first().unwrap();
    let current_leg = app.world().get::<RouteLeg>(current_leg_entity).unwrap();
    assert_eq!(current_leg.start_block.block_id, block_b);
    assert_eq!(current_leg.target_block.block_id, block_b);
}

/// Test that the virtual driver stops when no next leg is queued.
#[test]
fn virtual_driver_stops_without_next_leg() {
    let mut app = make_app();
    spawn_two_block_layout(&mut app);

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);
    let logical_graph = app.world().resource::<LogicalGraph>();

    let logical_a = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let logical_b = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    // Build idle + route WITHOUT trailing idle (train should stop at B)
    let idle = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map).unwrap();
    let route_legs = build_route(
        logical_a,
        logical_b,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .unwrap();

    // Spawn VirtualDriver as a separate entity
    app.world_mut()
        .spawn(VirtualDriver::new(TrainID(0), 1_000_000.0));

    // Place train and append route (no trailing idle)
    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), vec![idle]));
    app.update();
    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), route_legs));

    for _ in 0..20 {
        app.update();
    }

    // Train should have completed the route leg and be stuck at EnteredTarget
    // with no trailing idle to advance to.
    let train_entity = app
        .world()
        .resource::<Registry<Train>>()
        .get(&TrainID(0))
        .unwrap();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);

    // Only the route leg should remain (no advance happened)
    let legs = app.world().get::<TrainLegs>(train_entity).unwrap();
    assert_eq!(legs.collection().len(), 1);
    let current_leg = app
        .world()
        .get::<RouteLeg>(*legs.collection().first().unwrap())
        .unwrap();
    assert_eq!(current_leg.start_block.block_id, block_a);
    assert_eq!(current_leg.target_block.block_id, block_b);
}
