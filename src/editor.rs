use crate::layout::Layout;
use crate::layout_primitives::*;
use crate::section::TrackSection;
use crate::utils::bresenham_line;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};

#[derive(Component)]
enum GenericID {
    Cell(CellID),
    Track(TrackID),
    Block(BlockID),
    Train(TrainID),
    Switch(DirectedTrackID),
}

#[derive(Default)]
enum Selection {
    #[default]
    None,
    Single(GenericID),
    Multi(Vec<GenericID>),
}

#[derive(Resource, Default)]
struct SelectionState {
    selection: Selection,
    drag_select: bool,
}

#[derive(Resource, Default)]
struct HoverState {
    hover: Option<GenericID>,
}

#[derive(Resource, Default)]
struct TrackBuildState {
    hover_cells: Vec<CellID>,
    hover_track: Option<TrackID>,
}

impl TrackBuildState {
    fn build(&mut self, layout: &mut Layout, commands: &mut Commands) {
        while self.hover_cells.len() > 2 {
            if let Some(track_id) = TrackID::from_cells(
                self.hover_cells[0],
                self.hover_cells[1],
                self.hover_cells[2],
            ) {
                if !layout.has_track(track_id) {
                    commands.spawn(TrackBundle::new(track_id));
                    layout.add_track(track_id);
                }
                if let Some(track_b) = self.hover_track {
                    if let Some(connection) = track_b.get_connection_to(track_id) {
                        layout.connect_tracks(connection);
                    }
                }
                self.hover_track = Some(track_id);
            }
            self.hover_cells.remove(0);
        }
    }
}

#[derive(Component)]
struct Track {
    id: TrackID,
}

#[derive(Component, Default)]
struct Selectable {
    selected: bool,
}

#[derive(Bundle)]
struct TrackBundle {
    selectable: Selectable,
    track: Track,
    id: GenericID,
    name: Name,
}

impl TrackBundle {
    pub fn new(track_id: TrackID) -> Self {
        Self {
            id: GenericID::Track(track_id),
            track: Track { id: track_id },
            selectable: Selectable::default(),
            name: Name::new(format!("{:}", track_id)),
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    let pancam = PanCam {
        grab_buttons: vec![MouseButton::Middle],
        ..default()
    };
    commands
        .spawn((Camera2dBundle::default(), pancam))
        .add(InitWorldTracking)
        .insert(MainCamera);
}

fn init_draw_track(
    layout: Res<Layout>,
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_buttons: Res<Input<MouseButton>>,
    mouse_world_pos: Res<MousePosWorld>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        let first_cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
        track_build_state.hover_cells.push(first_cell);
    }
}

fn exit_draw_track(
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_buttons: Res<Input<MouseButton>>,
) {
    if mouse_buttons.just_released(MouseButton::Right) {
        track_build_state.hover_cells = vec![];
        track_build_state.hover_track = None;
    }
}

fn update_draw_track(
    mut layout: ResMut<Layout>,
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_world_pos: Res<MousePosWorld>,
    mut commands: Commands,
) {
    let last_cell = track_build_state.hover_cells.last();
    if last_cell.is_none() {
        return;
    }
    let start = (last_cell.unwrap().x, last_cell.unwrap().y);
    let mouse_cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
    for point in bresenham_line(start, (mouse_cell.x, mouse_cell.y)).iter() {
        let cell = CellID::new(point.0, point.1, 0);
        track_build_state.hover_cells.push(cell);
        // println!("{:?}", track_build_state.hover_cells);
        track_build_state.build(&mut layout, &mut commands);
    }
}

fn draw_build_cells(
    track_build_state: Res<TrackBuildState>,
    layout: Res<Layout>,
    mut gizmos: Gizmos,
    mouse_world_pos: Res<MousePosWorld>,
) {
    for cell in track_build_state.hover_cells.iter() {
        gizmos.circle_2d(
            cell.get_vec2() * layout.scale,
            layout.scale * 0.25,
            Color::GRAY,
        );
    }
    let cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
    gizmos.circle_2d(
        cell.get_vec2() * layout.scale,
        layout.scale * 0.25,
        Color::RED,
    );

    let scale = layout.scale;

    if let Some(track) = track_build_state.hover_track {
        for dirtrack in track.dirtracks() {
            dirtrack.draw_with_gizmos(&mut gizmos, scale, Color::RED)
        }
    }
}

fn init_select(
    buttons: Res<Input<MouseButton>>,
    mut mouse_world_pos: ResMut<MousePosWorld>,
    q_selectable: Query<Entity, &GenericID>,
    selection_state: Res<SelectionState>,
) {
    if buttons.just_pressed(MouseButton::Left) {}
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanCamPlugin);
        app.add_plugins(MousePosPlugin);
        app.add_plugins(WorldInspectorPlugin::default());
        app.insert_resource(TrackBuildState::default());
        app.insert_resource(HoverState::default());
        app.insert_resource(SelectionState::default());
        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            Update,
            (
                init_draw_track,
                exit_draw_track,
                update_draw_track,
                draw_build_cells,
                init_select,
            ),
        );
    }
}
