use crate::layout::Layout;
use crate::layout_primitives::*;
use crate::utils::bresenham_line;
use bevy::prelude::*;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};

#[derive(Resource, Default)]
struct TrackBuildState {
    hover_cells: Vec<CellID>,
}

impl TrackBuildState {
    fn build(&mut self, layout: &mut Layout) {
        while self.hover_cells.len() > 2 {
            if let Some(track) = TrackID::from_cells(
                self.hover_cells[0],
                self.hover_cells[1],
                self.hover_cells[2],
            ) {
                layout.add_track(track);
            }
            self.hover_cells.remove(0);
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    let pancam = PanCam {
        grab_buttons: vec![MouseButton::Middle, MouseButton::Left],
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
        track_build_state.hover_cells = vec![]
    }
}

fn update_draw_track(
    mut layout: ResMut<Layout>,
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_world_pos: Res<MousePosWorld>,
) {
    let last_cell = track_build_state.hover_cells.last();
    if last_cell.is_none() {
        return;
    }
    let start = (last_cell.unwrap().x, last_cell.unwrap().y);
    let mouse_cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
    if mouse_cell == *last_cell.unwrap() {
        return;
    }
    for point in bresenham_line(start, (mouse_cell.x, mouse_cell.y)).iter() {
        let cell = CellID::new(point.0, point.1, 0);
        track_build_state.hover_cells.push(cell);
        println!("{:?}", track_build_state.hover_cells);
        track_build_state.build(&mut layout);
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
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PanCamPlugin);
        app.add_plugins(MousePosPlugin);
        app.insert_resource(TrackBuildState::default());
        app.add_systems(Startup, spawn_camera);
        app.add_systems(
            Update,
            (
                init_draw_track,
                exit_draw_track,
                update_draw_track,
                draw_build_cells,
            ),
        );
    }
}
