use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use brickrail_common::block::{Block, BlockData};
use brickrail_common::command::{
    AppCommand, AppCommandPlugin, AppCommandQueue, CommandPlugin, CommandRegistry,
    EnterControlModeRequest, PlaceTrainAtBlockRequest, SendTrainToBlockRequest, SimulationCommand,
    SubAppClientPlugin,
};
use brickrail_common::layout::{Layout, LayoutAppPlugin};
use brickrail_common::layout_primitives::*;
use brickrail_common::lifecycle::{ElementData, ElementEntry, ElementId};
use brickrail_common::marker::{Marker, MarkerData};
use brickrail_common::route::{RouteLeg, TrainLegs};
use brickrail_common::track::Track;
use brickrail_common::train_position::TrainPosition;

const LAYOUT_SCALE: f32 = 40.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanCamPlugin)
        .add_plugins(LayoutAppPlugin)
        .add_plugins(ClientSimulationPlugin)
        .add_systems(Startup, (spawn_camera, queue_init_commands))
        .add_systems(
            Update,
            (draw_tracks, draw_markers, draw_blocks, draw_trains),
        )
        .run();
}

/// Plugin that sets up the simulation SubApp and client-side command handling.
struct ClientSimulationPlugin;

impl Plugin for ClientSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CommandPlugin);
        app.add_plugins(AppCommandPlugin);
        app.add_plugins(SubAppClientPlugin);
    }
}

/// Queue all init commands at startup. The AppCommandQueue executes them sequentially.
fn queue_init_commands(
    mut queue: ResMut<AppCommandQueue>,
    mut registry: ResMut<CommandRegistry>,
    mut commands: Commands,
) {
    let layout = test_layout();

    let t0 = TrackID::new(CellID::new(0, 2, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 2, 0), Orientation::EW);
    let t7 = TrackID::new(CellID::new(2, -1, 0), Orientation::EW);
    let t8 = TrackID::new(CellID::new(1, -1, 0), Orientation::EW);

    let logical_a = LogicalBlockID {
        block: BlockID::new(t0, t1),
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let logical_b = LogicalBlockID {
        block: BlockID::new(t7, t8),
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    // 1. Spawn layout in main world (for rendering).
    queue.push(
        &mut commands,
        &mut registry,
        AppCommand::SpawnLayout(layout.clone()),
    );

    // 2. Enter control mode — syncs layout to SubApp + spawns VirtualDrivers.
    queue.push(
        &mut commands,
        &mut registry,
        AppCommand::Simulation(SimulationCommand::EnterControlMode(
            EnterControlModeRequest { layout },
        )),
    );

    // 3. Place train at block A.
    queue.push(
        &mut commands,
        &mut registry,
        AppCommand::Simulation(SimulationCommand::PlaceTrainAtBlock(
            PlaceTrainAtBlockRequest {
                train: TrainID(0),
                block: logical_a,
            },
        )),
    );

    // 4. Send train from A to B.
    queue.push(
        &mut commands,
        &mut registry,
        AppCommand::Simulation(SimulationCommand::SendTrainToBlock(
            SendTrainToBlockRequest {
                train: TrainID(0),
                target_block: logical_b,
            },
        )),
    );
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d::default(), PanCam::default()));
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
            ElementEntry::new(
                t0,
                MarkerData {
                    color: MarkerColor::Red,
                },
            ),
            ElementEntry::new(
                t1,
                MarkerData {
                    color: MarkerColor::Blue,
                },
            ),
            ElementEntry::new(
                t4,
                MarkerData {
                    color: MarkerColor::Green,
                },
            ),
            ElementEntry::new(
                t7,
                MarkerData {
                    color: MarkerColor::Yellow,
                },
            ),
            ElementEntry::new(
                t8,
                MarkerData {
                    color: MarkerColor::Cyan,
                },
            ),
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
