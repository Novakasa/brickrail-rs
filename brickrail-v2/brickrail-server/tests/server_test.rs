use bevy::prelude::*;
use brickrail_common::layout::*;
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::*;
use brickrail_common::block::Block;
use brickrail_common::connection::{Connection, ConnectionGraph};
use brickrail_common::block::BlockData;
use brickrail_common::logical_graph::{LogicalGraph, LogicalGraphPlugin};
use brickrail_common::marker::{Marker, MarkerData};
use brickrail_common::route::{AppendLegs, LegOf, MarkerRole, RouteLeg, TrainLegs};
use bevy::ecs::relationship::RelationshipTarget;
use brickrail_common::train_position::{AdvanceLeg, TrainLegState, TrainMarkerHit, TrainPosition};
use brickrail_common::simulation::SimulationStatePlugin;
use petgraph::algo::astar;
use bevy::platform::collections::HashMap;
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

// --- Plugin isolation tests ---
// These verify that plugin dependency chains are clean:
// each tier works without higher tiers present.

#[test]
fn tracks_only_app_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutInstancePlugin::<ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Track, ServerLayout>::new());
    app.update(); // should not panic
}

#[test]
fn full_layout_without_simulation_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutInstancePlugin::<ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Track, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Connection, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Marker, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Block, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Train, ServerLayout>::new());
    app.add_plugins(LogicalGraphPlugin::<ServerLayout>::new());
    app.update(); // should not panic — no SimulationStatePlugin needed
}

#[test]
fn simulation_state_without_logic_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutInstancePlugin::<ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Track, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Connection, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Marker, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Block, ServerLayout>::new());
    app.add_plugins(ElementPlugin::<Train, ServerLayout>::new());
    app.add_plugins(LogicalGraphPlugin::<ServerLayout>::new());
    app.add_plugins(SimulationStatePlugin::<ServerLayout>::new());
    app.update(); // should not panic — no SimulationLogicPlugin needed
}

/// Test helper: A* pathfinding + leg building in one call.
fn build_route(
    start: LogicalBlockID,
    target: LogicalBlockID,
    logical_graph: &LogicalGraph<ServerLayout>,
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

    // Logical graph should be built (runs in Last schedule)
    // 3 tracks × 2 directions × 2 facings = 12 logical track nodes
    // But only nodes with edges are in the graph.
    // 2 connections × 4 directed edges = 8 normal edges
    // 1 block × 2 flip edges (bidirectional) = 4 flip edges
    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();
    assert_eq!(logical_graph.graph.edge_count(), 12);

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

/// Layout with two blocks and a travel section between them:
/// t0 -- t1 -- t2 -- t3 -- t4
/// [block A]  travel  [block B]
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

#[test]
fn build_route_between_two_blocks() {
    let mut app = make_app();
    let layout = two_block_layout();

    app.world_mut()
        .write_message(EnterControlMode { layout });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    // Build block data map
    let block_registry = app.world().resource::<Registry<Block, ServerLayout>>();
    let mut block_data_map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = app.world().get::<ElementData<Block>>(entity).unwrap();
        block_data_map.insert(*id, data.0.clone());
    }

    // Build marker data map
    let marker_registry = app.world().resource::<Registry<Marker, ServerLayout>>();
    let mut marker_data_map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = app.world().get::<ElementData<Marker>>(entity).unwrap();
        marker_data_map.insert(*id, data.0.clone());
    }

    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();

    let start = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let target = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    let legs = build_route(start, target, logical_graph, &block_data_map, &marker_data_map)
        .expect("should find a route");

    assert_eq!(legs.len(), 1);

    let leg = &legs[0];
    assert_eq!(leg.facing, Facing::Forward);
    assert_eq!(leg.start_block.block_id, block_a);
    assert_eq!(leg.target_block.block_id, block_b);

    // Start block section should contain the block A tracks
    assert_eq!(leg.start_block.section.len(), 2);
    assert_eq!(leg.start_block.section[0].track, t0);
    assert_eq!(leg.start_block.section[1].track, t1);

    // Travel section should contain t2
    assert_eq!(leg.travel.len(), 1);
    assert_eq!(leg.travel[0].track, t2);

    // Target block section should contain block B tracks
    assert_eq!(leg.target_block.section.len(), 2);
    assert_eq!(leg.target_block.section[0].track, t3);
    assert_eq!(leg.target_block.section[1].track, t4);

    // Markers along sensor trajectory: t1 (Exiting), t3 (Entering), t4 (Entered)
    // t0 is not on the sensor path (A* starts at enter marker t1)
    // t2 has no marker in this layout
    assert_eq!(leg.markers.len(), 3);
    assert_eq!(leg.markers[0].track, t1);
    assert_eq!(leg.markers[0].role, Some(MarkerRole::Exiting));
    assert_eq!(leg.markers[1].track, t3);
    assert_eq!(leg.markers[1].role, Some(MarkerRole::Entering));
    assert_eq!(leg.markers[2].track, t4);
    assert_eq!(leg.markers[2].role, Some(MarkerRole::Entered));

    // Positions should span 0.0 to 1.0
    assert_eq!(leg.markers[0].position, 0.0);
    assert_eq!(leg.markers[2].position, 1.0);
}

#[test]
fn append_legs_spawns_leg_entities() {
    let mut app = make_app();
    let layout = two_block_layout();

    app.world_mut()
        .write_message(EnterControlMode { layout });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    // Build legs externally (as strategy code would)
    let block_registry = app.world().resource::<Registry<Block, ServerLayout>>();
    let mut block_data_map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = app.world().get::<ElementData<Block>>(entity).unwrap();
        block_data_map.insert(*id, data.0.clone());
    }
    let marker_registry = app.world().resource::<Registry<Marker, ServerLayout>>();
    let mut marker_data_map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = app.world().get::<ElementData<Marker>>(entity).unwrap();
        marker_data_map.insert(*id, data.0.clone());
    }
    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();

    let start = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let target = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    let legs = build_route(start, target, logical_graph, &block_data_map, &marker_data_map)
        .expect("should find a route");

    // Append the legs via state event message
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), legs));
    app.update();

    // Resolve train entity from registry for assertions
    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();

    // Query spawned leg entities
    let spawned: Vec<_> = app
        .world_mut()
        .query::<(&RouteLeg, &LegOf)>()
        .iter(app.world())
        .collect();

    assert_eq!(spawned.len(), 1);

    let (leg, leg_of) = spawned[0];
    assert_eq!(leg_of.0, train_entity);
    assert_eq!(leg.start_block.block_id, block_a);
    assert_eq!(leg.target_block.block_id, block_b);
    assert_eq!(leg.facing, Facing::Forward);
    assert_eq!(leg.markers.len(), 3);
}

// --- Helper: extract data maps from ECS ---

fn extract_block_data_map(app: &App) -> HashMap<BlockID, BlockData> {
    let block_registry = app.world().resource::<Registry<Block, ServerLayout>>();
    let mut map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = app.world().get::<ElementData<Block>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn extract_marker_data_map(app: &App) -> HashMap<TrackID, MarkerData> {
    let marker_registry = app.world().resource::<Registry<Marker, ServerLayout>>();
    let mut map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = app.world().get::<ElementData<Marker>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

// --- Train position tests ---

#[test]
fn append_idle_creates_position() {
    let mut app = make_app();
    app.world_mut()
        .write_message(EnterControlMode { layout: two_block_layout() });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);

    let logical_block = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    let idle_leg = RouteLeg::idle(logical_block, &block_data_map, &marker_data_map)
        .expect("should build idle leg");

    // Idle leg: start == target, no travel, one marker (Entered — always assigned to last marker)
    assert_eq!(idle_leg.start_block.block_id, block_a);
    assert_eq!(idle_leg.target_block.block_id, block_a);
    assert!(idle_leg.travel.is_empty());
    assert_eq!(idle_leg.markers.len(), 1);
    assert_eq!(idle_leg.markers[0].role, Some(MarkerRole::Entered));

    // Append the idle leg
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), vec![idle_leg]));
    app.update();

    // TrainPosition should now exist
    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = app.world().get::<TrainPosition>(train_entity)
        .expect("TrainPosition should exist after first AppendLegs");

    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
    assert_eq!(position.marker_index, 0);
}

#[test]
fn advance_off_idle_to_route() {
    let mut app = make_app();
    app.world_mut()
        .write_message(EnterControlMode { layout: two_block_layout() });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);
    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();

    // Build idle leg at block A
    let logical_a = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let idle_leg = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map)
        .expect("should build idle leg");

    // Build route legs A → B + trailing idle at B
    let logical_b = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let route_legs = build_route(logical_a, logical_b, logical_graph, &block_data_map, &marker_data_map)
        .expect("should find route");
    let trailing_idle = RouteLeg::idle(logical_b, &block_data_map, &marker_data_map)
        .expect("should build trailing idle");

    // Append idle, then route + trailing idle
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), vec![idle_leg]));
    app.update();

    let mut route_with_trailing = route_legs;
    route_with_trailing.push(trailing_idle);
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), route_with_trailing));
    app.update();

    // Advance off idle
    app.world_mut()
        .write_message(AdvanceLeg::<ServerLayout>::new(TrainID(0)));
    app.update();

    // TrainPosition should point to the route leg, not idle
    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();

    assert_eq!(position.leg_state, TrainLegState::ExitingStart);
    assert_eq!(position.marker_index, 0);

    // The current leg (first in TrainLegs) should be the route leg (A → B)
    let legs = app.world().get::<TrainLegs>(train_entity).unwrap();
    let current_leg_entity = *legs.collection().first().unwrap();
    let current_leg = app.world().get::<RouteLeg>(current_leg_entity).unwrap();
    assert_eq!(current_leg.start_block.block_id, block_a);
    assert_eq!(current_leg.target_block.block_id, block_b);

    // Should have 3 legs total now (idle despawned, route + trailing idle remain)
    let legs_count = app.world_mut()
        .query::<&RouteLeg>()
        .iter(app.world())
        .count();
    assert_eq!(legs_count, 2); // route leg + trailing idle
}

#[test]
fn marker_hit_increments_and_updates_state() {
    let mut app = make_app();
    app.world_mut()
        .write_message(EnterControlMode { layout: two_block_layout() });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);
    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();

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

    // Place train with idle, append route + trailing idle, advance off idle
    let idle = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map).unwrap();
    let mut legs = build_route(logical_a, logical_b, logical_graph, &block_data_map, &marker_data_map).unwrap();
    legs.push(RouteLeg::idle(logical_b, &block_data_map, &marker_data_map).unwrap());

    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), vec![idle]));
    app.update();
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), legs));
    app.update();
    app.world_mut()
        .write_message(AdvanceLeg::<ServerLayout>::new(TrainID(0)));
    app.update();

    // Route leg has 3 markers: [0]=Exiting, [1]=Entering, [2]=Entered
    // Train starts at marker_index=0 (already past the Exiting marker).
    // Each TrainMarkerHit increments marker_index and reads the new marker's role.

    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();

    // First hit: marker_index 0→1, markers[1].role = Entering
    app.world_mut()
        .write_message(TrainMarkerHit::<ServerLayout>::new(TrainID(0)));
    app.update();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.marker_index, 1);
    assert_eq!(position.leg_state, TrainLegState::EnteringTarget);

    // Second hit: marker_index 1→2, markers[2].role = Entered
    app.world_mut()
        .write_message(TrainMarkerHit::<ServerLayout>::new(TrainID(0)));
    app.update();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();
    assert_eq!(position.marker_index, 2);
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
}

#[test]
fn advance_leg_moves_to_next() {
    let mut app = make_app();
    app.world_mut()
        .write_message(EnterControlMode { layout: two_block_layout() });
    app.update();

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let block_data_map = extract_block_data_map(&app);
    let marker_data_map = extract_marker_data_map(&app);
    let logical_graph = app.world().resource::<LogicalGraph<ServerLayout>>();

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

    // Build: idle at A, route A→B, trailing idle at B
    let idle = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map).unwrap();
    let mut legs = build_route(logical_a, logical_b, logical_graph, &block_data_map, &marker_data_map).unwrap();
    legs.push(RouteLeg::idle(logical_b, &block_data_map, &marker_data_map).unwrap());

    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), vec![idle]));
    app.update();
    app.world_mut()
        .write_message(AppendLegs::<ServerLayout>::new(TrainID(0), legs));
    app.update();

    // Advance off idle → route leg
    app.world_mut()
        .write_message(AdvanceLeg::<ServerLayout>::new(TrainID(0)));
    app.update();

    // Advance off route leg → trailing idle
    app.world_mut()
        .write_message(AdvanceLeg::<ServerLayout>::new(TrainID(0)));
    app.update();

    let train_registry = app.world().resource::<Registry<Train, ServerLayout>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();
    let position = app.world().get::<TrainPosition>(train_entity).unwrap();

    // Should be on trailing idle at block B
    assert_eq!(position.leg_state, TrainLegState::EnteredTarget);
    assert_eq!(position.marker_index, 0);

    // Current leg (first in TrainLegs) should be the trailing idle
    let legs = app.world().get::<TrainLegs>(train_entity).unwrap();
    let current_leg_entity = *legs.collection().first().unwrap();
    let current_leg = app.world().get::<RouteLeg>(current_leg_entity).unwrap();
    assert_eq!(current_leg.start_block.block_id, block_b);
    assert_eq!(current_leg.target_block.block_id, block_b);
    assert!(current_leg.travel.is_empty());

    // Only 1 leg entity remaining (the trailing idle)
    let legs_count = app.world_mut()
        .query::<&RouteLeg>()
        .iter(app.world())
        .count();
    assert_eq!(legs_count, 1);
}
