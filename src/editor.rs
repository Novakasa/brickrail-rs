use crate::layout::Layout;
use crate::layout_primitives::*;
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
    if let Some(last_cell) = track_build_state.hover_cells.last() {
        let cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
        if cell != *last_cell {
            track_build_state.hover_cells.push(cell);
            println!("{:?}", track_build_state.hover_cells);
            track_build_state.build(&mut layout);
        }
    }
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
            (init_draw_track, exit_draw_track, update_draw_track),
        );
    }
}
