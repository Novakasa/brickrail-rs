use bevy::app::{Main, MainSchedulePlugin};
use bevy::ecs::relationship::RelationshipTarget;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use brickrail_common::block::{Block, BlockData};
use brickrail_common::command::{
    CommandPlugin, CommandResponse, SimulationCommandPlugin, SubAppClientPlugin,
    SubAppCommandInputQueue, SubAppCommandResponseQueue, SubAppServerPlugin,
};
use brickrail_common::connection::Connection;
use brickrail_common::layout::{Layout, LayoutAppPlugin, LayoutSubApp};
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::{ElementData, ElementEntry, ElementId, SpawnElement};
use brickrail_common::logical_graph::LogicalGraph;
use brickrail_common::marker::{Marker, MarkerData};
use brickrail_common::route::{AppendLegs, RouteLeg, TrainLegs};
use brickrail_common::simulation::{SimulationLogicPlugin, SimulationSet};
use brickrail_common::simulation_event::{SimulationEvent, SimulationEventQueue};
use brickrail_common::track::Track;
use brickrail_common::train::Train;
use brickrail_common::train_position::TrainPosition;
use brickrail_common::virtual_driver::{VirtualDriver, VirtualDriverPlugin};
use petgraph::algo::astar;

const LAYOUT_SCALE: f32 = 40.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanCamPlugin)
        .add_plugins(LayoutAppPlugin)
        .add_plugins(ClientSimulationPlugin)
        .add_systems(Startup, (spawn_camera, spawn_layout))
        .add_systems(
            Update,
            (draw_tracks, draw_markers, draw_blocks, draw_trains),
        )
        .run();
}

/// Plugin that sets up the simulation SubApp and extract bridge.
struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        let layout = test_layout();

        let mut sub_app = SubApp::new();
        sub_app.update_schedule = Some(Main.intern());

        // Bootstrap SubApp with standard Bevy schedules and message plumbing.
        sub_app.add_plugins(MainSchedulePlugin);
        sub_app.add_systems(
            First,
            bevy::ecs::message::message_update_system
                .in_set(bevy::ecs::message::MessageUpdateSystems)
                .run_if(bevy::ecs::message::message_update_condition),
        );

        sub_app.init_resource::<bevy::ecs::reflect::AppTypeRegistry>();
        sub_app.add_plugins(LayoutAppPlugin);
        sub_app.add_plugins(SimulationLogicPlugin);
        sub_app.add_plugins(SimulationCommandPlugin);
        sub_app.add_plugins(SubAppServerPlugin);
        sub_app.add_plugins(bevy::time::TimePlugin);
        sub_app.add_plugins(VirtualDriverPlugin);

        // Spawn layout elements in the SubApp.
        spawn_layout_into(sub_app.world_mut(), &layout);

        // Init resource: route building + simulation kickoff happens on second frame.
        sub_app.insert_resource(SimulationInitNeeded(true));
        sub_app.add_systems(
            Update,
            init_simulation
                .run_if(|r: Res<SimulationInitNeeded>| r.0)
                .in_set(SimulationSet::Logic),
        );

        // Bidirectional extract bridge.
        sub_app.set_extract(|main_world, sub_world| {
            // Output: simulation events → main app.
            let mut event_queue = sub_world.resource_mut::<SimulationEventQueue>();
            let events: Vec<SimulationEvent> = event_queue.0.drain(..).collect();
            for event in events {
                main_world.write_message(event);
            }

            // Output: command responses → main app.
            let mut response_queue = sub_world.resource_mut::<SubAppCommandResponseQueue>();
            let responses: Vec<CommandResponse> = response_queue.0.drain(..).collect();
            for response in responses {
                main_world.write_message(response);
            }

            // Input: simulation commands → SubApp.
            let mut cmd_queue = main_world.resource_mut::<SubAppCommandInputQueue>();
            let commands: Vec<_> = cmd_queue.0.drain(..).collect();
            for cmd in commands {
                sub_world.write_message(cmd);
            }
        });

        app.add_plugins(CommandPlugin);
        app.add_plugins(SubAppClientPlugin);
        app.insert_sub_app(LayoutSubApp, sub_app);
    }
}

/// Write SpawnElement messages for all layout elements into a world.
fn spawn_layout_into(world: &mut World, layout: &Layout) {
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

#[derive(Resource)]
struct SimulationInitNeeded(bool);

/// One-time init: build route and kick off simulation.
/// Waits until layout elements are spawned and LogicalGraph is built.
fn init_simulation(
    mut needs_init: ResMut<SimulationInitNeeded>,
    logical_graph: Res<LogicalGraph>,
    block_registry: Res<brickrail_common::lifecycle::Registry<Block>>,
    block_data_query: Query<&ElementData<Block>>,
    marker_registry: Res<brickrail_common::lifecycle::Registry<Marker>>,
    marker_data_query: Query<&ElementData<Marker>>,
    mut commands: Commands,
    mut event_writer: MessageWriter<SimulationEvent>,
) {
    // Wait until registries are populated and LogicalGraph is built.
    if block_registry.is_empty()
        || marker_registry.is_empty()
        || logical_graph.graph.node_count() == 0
    {
        return;
    }

    // Build data maps from ECS.
    let mut block_data_map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        if let Ok(data) = block_data_query.get(entity) {
            block_data_map.insert(*id, data.0.clone());
        }
    }
    let mut marker_data_map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        if let Ok(data) = marker_data_query.get(entity) {
            marker_data_map.insert(*id, data.0.clone());
        }
    }

    // Use blocks A and B from test_layout.
    let t0 = TrackID::new(CellID::new(0, 2, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 2, 0), Orientation::EW);
    let t7 = TrackID::new(CellID::new(2, -1, 0), Orientation::EW);
    let t8 = TrackID::new(CellID::new(1, -1, 0), Orientation::EW);
    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t7, t8);

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

    // Build idle at A, route A→B, trailing idle at B.
    let idle = RouteLeg::idle(logical_a, &block_data_map, &marker_data_map)
        .expect("should build idle leg at A");

    let start_data = block_data_map.get(&block_a).unwrap();
    let target_data = block_data_map.get(&block_b).unwrap();
    let start_track = start_data.enter_logical_track(logical_a.direction, logical_a.facing);
    let target_track = target_data.enter_logical_track(logical_b.direction, logical_b.facing);

    let (_cost, path) = astar(
        &logical_graph.graph,
        start_track,
        |n| n == target_track,
        |_| 1u32,
        |_| 0u32,
    )
    .expect("should find path A→B");

    let mut route_legs =
        RouteLeg::build_from_path(&path, &block_data_map, &marker_data_map)
            .expect("should build route legs");
    let trailing_idle = RouteLeg::idle(logical_b, &block_data_map, &marker_data_map)
        .expect("should build trailing idle at B");
    route_legs.push(trailing_idle);

    // Combine: idle + route + trailing idle.
    let mut all_legs = vec![idle];
    all_legs.extend(route_legs);

    // Spawn VirtualDriver (separate entity, SubApp only).
    commands.spawn(VirtualDriver::new(TrainID(0), 1.0));

    // Write AppendLegs via SimulationEvent so it gets collected and forwarded to client.
    event_writer.write(SimulationEvent::AppendLegs(AppendLegs::new(
        TrainID(0),
        all_legs,
    )));

    // Mark init as done so this system doesn't run again.
    needs_init.0 = false;
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d::default(), PanCam::default()));
}

fn spawn_layout(
    mut spawn_tracks: MessageWriter<SpawnElement<Track>>,
    mut spawn_connections: MessageWriter<SpawnElement<Connection>>,
    mut spawn_markers: MessageWriter<SpawnElement<Marker>>,
    mut spawn_blocks: MessageWriter<SpawnElement<Block>>,
    mut spawn_trains: MessageWriter<SpawnElement<Train>>,
) {
    let layout = test_layout();

    for entry in &layout.tracks {
        spawn_tracks.write(SpawnElement::from_entry(entry));
    }
    for entry in &layout.connections {
        spawn_connections.write(SpawnElement::from_entry(entry));
    }
    for entry in &layout.markers {
        spawn_markers.write(SpawnElement::from_entry(entry));
    }
    for entry in &layout.blocks {
        spawn_blocks.write(SpawnElement::from_entry(entry));
    }
    for entry in &layout.trains {
        spawn_trains.write(SpawnElement::from_entry(entry));
    }
}

/// Build a test layout: C-shaped path with two blocks.
///
/// ```text
///   t0 -- t1 -- t2 -- t3
///                       |
///                      t4
///                       |
///                      t5
///                       |
///                  t7 -- t8
/// ```
fn test_layout() -> Layout {
    // Horizontal segment (y=2)
    let t0 = TrackID::new(CellID::new(0, 2, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 2, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 2, 0), Orientation::EW);
    // Corner
    let t3 = TrackID::new(CellID::new(3, 2, 0), Orientation::SW);
    // Vertical segment
    let t4 = TrackID::new(CellID::new(3, 1, 0), Orientation::NS);
    let t5 = TrackID::new(CellID::new(3, 0, 0), Orientation::NS);
    // Bottom horizontal segment
    let t6 = TrackID::new(CellID::new(3, -1, 0), Orientation::NW);
    let t7 = TrackID::new(CellID::new(2, -1, 0), Orientation::EW);
    let t8 = TrackID::new(CellID::new(1, -1, 0), Orientation::EW);

    Layout {
        tracks: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t2, Default::default()),
            ElementEntry::new(t3, Default::default()),
            ElementEntry::new(t4, Default::default()),
            ElementEntry::new(t5, Default::default()),
            ElementEntry::new(t6, Default::default()),
            ElementEntry::new(t7, Default::default()),
            ElementEntry::new(t8, Default::default()),
        ],
        connections: vec![
            ElementEntry::new(t0.get_connection_to(t1).unwrap(), Default::default()),
            ElementEntry::new(t1.get_connection_to(t2).unwrap(), Default::default()),
            ElementEntry::new(t2.get_connection_to(t3).unwrap(), Default::default()),
            ElementEntry::new(t3.get_connection_to(t4).unwrap(), Default::default()),
            ElementEntry::new(t4.get_connection_to(t5).unwrap(), Default::default()),
            ElementEntry::new(t5.get_connection_to(t6).unwrap(), Default::default()),
            ElementEntry::new(t6.get_connection_to(t7).unwrap(), Default::default()),
            ElementEntry::new(t7.get_connection_to(t8).unwrap(), Default::default()),
        ],
        markers: vec![
            ElementEntry::new(t0, MarkerData { color: MarkerColor::Red }),
            ElementEntry::new(t1, MarkerData { color: MarkerColor::Blue }),
            ElementEntry::new(t4, MarkerData { color: MarkerColor::Green }),
            ElementEntry::new(t7, MarkerData { color: MarkerColor::Yellow }),
            ElementEntry::new(t8, MarkerData { color: MarkerColor::Cyan }),
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
                BlockID::new(t7, t8),
                BlockData {
                    name: Some("B".to_string()),
                    section: vec![
                        t7.get_directed_to(Cardinal::W).unwrap(),
                        t8.get_directed_to(Cardinal::W).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
        ],
        trains: vec![ElementEntry::new(TrainID(0), Default::default())],
    }
}

fn draw_tracks(mut gizmos: Gizmos, query: Query<&ElementId<Track>>) {
    for id in &query {
        let (a, b) = id.0.slot_positions();
        gizmos.line_2d(a * LAYOUT_SCALE, b * LAYOUT_SCALE, Color::WHITE);
    }
}

fn draw_markers(mut gizmos: Gizmos, query: Query<(&ElementId<Marker>, &ElementData<Marker>)>) {
    for (id, data) in &query {
        let pos = id.0.cell.get_vec2() * LAYOUT_SCALE;
        let color = marker_color_to_color(data.0.color);
        gizmos.circle_2d(pos, 4.0, color);
    }
}

fn draw_blocks(mut gizmos: Gizmos, query: Query<&ElementData<Block>>) {
    let block_color = Color::srgba(0.0, 0.8, 0.8, 0.4);
    let block_width = 8.0;
    for data in &query {
        for directed_track in &data.0.section {
            let (a, b) = directed_track.track.slot_positions();
            let a = a * LAYOUT_SCALE;
            let b = b * LAYOUT_SCALE;
            let center = (a + b) * 0.5;
            let delta = b - a;
            let length = delta.length();
            let angle = delta.y.atan2(delta.x);
            gizmos.rect_2d(
                Isometry2d::new(center, Rot2::radians(angle)),
                Vec2::new(length, block_width),
                block_color,
            );
        }
    }
}

fn draw_trains(
    mut gizmos: Gizmos,
    query: Query<(&TrainPosition, &TrainLegs)>,
    leg_query: Query<&RouteLeg>,
) {
    for (position, legs) in &query {
        let Some(&leg_entity) = legs.collection().first() else {
            continue;
        };
        let Ok(leg) = leg_query.get(leg_entity) else {
            continue;
        };
        if position.marker_index >= leg.markers.len() {
            continue;
        }
        let marker = &leg.markers[position.marker_index];
        let pos = marker.track.cell.get_vec2() * LAYOUT_SCALE;
        gizmos.circle_2d(pos, 6.0, Color::srgb(1.0, 0.5, 0.0));
    }
}

fn marker_color_to_color(mc: MarkerColor) -> Color {
    match mc {
        MarkerColor::None => Color::srgb(0.5, 0.5, 0.5),
        MarkerColor::Red => Color::srgb(1.0, 0.0, 0.0),
        MarkerColor::Yellow => Color::srgb(1.0, 1.0, 0.0),
        MarkerColor::Green => Color::srgb(0.0, 1.0, 0.0),
        MarkerColor::Blue => Color::srgb(0.0, 0.0, 1.0),
        MarkerColor::Cyan => Color::srgb(0.0, 1.0, 1.0),
        MarkerColor::White => Color::WHITE,
        MarkerColor::Black => Color::BLACK,
    }
}
