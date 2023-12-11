use crate::layout::Layout;
use crate::layout_primitives::*;
use crate::section::TrackSection;
use crate::utils::bresenham_line;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mouse_tracking_plugin::{prelude::*, MainCamera, MousePosWorld};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum GenericID {
    Cell(CellID),
    Track(TrackID),
    Block(BlockID),
    Train(TrainID),
    Switch(DirectedTrackID),
}

#[derive(Default, Clone)]
enum Selection {
    #[default]
    None,
    Single(GenericID),
    Multi(Vec<GenericID>),
    Section(TrackSection),
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

fn build_connection_path(connection: TrackConnectionID) -> Path {
    let dirconnection = connection.to_directed(ConnectionDirection::Forward);
    let mut path_builder = PathBuilder::new();
    let length = dirconnection.connection_length();
    path_builder.move_to(dirconnection.interpolate_pos(0.0) * 40.0);
    let num_segments = match dirconnection.curve_index() {
        0 => 1,
        _ => 10,
    };
    for i in 1..(num_segments + 1) {
        let dist = i as f32 * length / num_segments as f32;
        path_builder.line_to(dirconnection.interpolate_pos(dist) * 40.0);
    }

    path_builder.build()
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
                    if let Some(connection_id) = track_b.get_connection_to(track_id) {
                        if !layout.has_connection(&connection_id) {
                            commands
                                .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Outer));
                            commands
                                .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Inner));
                            layout.connect_tracks_simple(&connection_id);
                        }
                    }
                }
                self.hover_track = Some(track_id);
            }
            self.hover_cells.remove(0);
        }
    }
}

#[derive(PartialEq, Eq)]
enum TrackShapeType {
    Outer,
    Inner,
}

#[derive(Component)]
struct TrackConnectionShape {
    id: TrackConnectionID,
    shape_type: TrackShapeType,
}

#[derive(Bundle)]
struct TrackBaseShape {
    connection: TrackConnectionShape,
    shape: ShapeBundle,
    stroke: Stroke,
}

impl TrackBaseShape {
    pub fn new(id: TrackConnectionID, shape_type: TrackShapeType) -> Self {
        let position = id.track_a().cell().get_vec2() * 40.0;

        let (color, width, z) = match &shape_type {
            TrackShapeType::Inner => (Color::BLACK, 6.0, 10.0),
            TrackShapeType::Outer => (Color::WHITE, 10.0, 5.0),
        };

        let connection = TrackConnectionShape {
            id: id,
            shape_type: shape_type,
        };
        Self {
            connection: connection,
            shape: ShapeBundle {
                path: build_connection_path(id),
                spatial: SpatialBundle {
                    transform: Transform::from_xyz(0.0, 0.0, z),
                    ..default()
                },
                ..default()
            },
            stroke: Stroke {
                color,
                options: StrokeOptions::default()
                    .with_line_width(width)
                    .with_line_cap(LineCap::Round),
            },
        }
    }
}

#[derive(Component)]
struct Track {
    id: TrackID,
}

#[derive(Component)]
struct Selectable {
    id: GenericID,
}

impl Selectable {
    pub fn new(id: GenericID) -> Self {
        Self { id: id }
    }

    fn signed_distance(&self, normalized_pos: Vec2) -> f32 {
        match self.id {
            GenericID::Track(track_id) => track_id.distance_to(normalized_pos) - 0.4,
            _ => 10.0,
        }
    }
}

#[derive(Bundle)]
struct TrackBundle {
    selectable: Selectable,
    track: Track,
    name: Name,
}

impl TrackBundle {
    pub fn new(track_id: TrackID) -> Self {
        Self {
            track: Track { id: track_id },
            selectable: Selectable::new(GenericID::Track(track_id)),
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
    hover_state: Res<HoverState>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        let first_cell = CellID::from_vec2(mouse_world_pos.truncate() / layout.scale);
        track_build_state.hover_cells.push(first_cell);
        if let Some(GenericID::Track(track_id)) = hover_state.hover {
            track_build_state.hover_track = Some(track_id);
        }
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

fn update_hover(
    mouse_world_pos: Res<MousePosWorld>,
    q_selectable: Query<&Selectable>,
    mut hover_state: ResMut<HoverState>,
) {
    hover_state.hover = None;
    let mut min_dist = f32::INFINITY;
    for selectable in q_selectable.iter() {
        let dist = selectable.signed_distance(mouse_world_pos.truncate() / 40.0);
        // println!("{:}", dist);
        if dist < min_dist && dist < 0.0 {
            hover_state.hover = Some(selectable.id);
            min_dist = dist;
        } else {
        }
    }
}

fn update_track_color(
    mut q_strokes: Query<(&TrackConnectionShape, &mut Stroke, &mut Transform)>,
    hover_state: Res<HoverState>,
) {
    for (connection, mut stroke, mut transform) in q_strokes.iter_mut() {
        if connection.shape_type == TrackShapeType::Outer {
            continue;
        }
        if hover_state.hover == Some(GenericID::Track(connection.id.track_a().track))
            || hover_state.hover == Some(GenericID::Track(connection.id.track_b().track))
        {
            stroke.color = Color::RED;
            transform.translation = Vec3::new(0.0, 0.0, 20.0);
        } else {
            stroke.color = Color::BLACK;
            transform.translation = Vec3::new(0.0, 0.0, 10.0);
        }
    }
}

fn init_select(
    buttons: Res<Input<MouseButton>>,
    hover_state: Res<HoverState>,
    mut selection_state: ResMut<SelectionState>,
) {
    if buttons.just_pressed(MouseButton::Left) {
        match hover_state.hover {
            Some(id) => match id {
                GenericID::Track(track_id) => {
                    let mut section = TrackSection::new();
                    section
                        .push(
                            track_id.get_directed(TrackDirection::Aligned),
                            &Layout::default(),
                        )
                        .unwrap();
                    selection_state.selection = Selection::Section(section);
                }
                _ => {}
            },
            None => {}
        }
    }
}

fn draw_selection(
    mut gizmos: Gizmos,
    selection_state: Res<SelectionState>,
    mouse_world_pos: Res<MousePosWorld>,
    layout: Res<Layout>,
) {
    match &selection_state.selection {
        Selection::Section(section) => {
            for track in section.tracks.iter() {
                track.draw_with_gizmos(&mut gizmos, layout.scale, Color::BLUE);
            }
        }
        _ => {}
    }
}

fn extend_selection(
    hover_state: Res<HoverState>,
    buttons: Res<Input<MouseButton>>,
    mut selection_state: ResMut<SelectionState>,
    layout: Res<Layout>,
) {
    if hover_state.is_changed() {
        if buttons.pressed(MouseButton::Left) {
            match (&hover_state.hover, &mut selection_state.selection) {
                (Some(GenericID::Track(track_id)), Selection::Section(section)) => {
                    match section.push_track(*track_id, &layout) {
                        Ok(()) => {}
                        Err(()) => {
                            let mut opposite = section.get_opposite();
                            match opposite.push_track(*track_id, &layout) {
                                Ok(()) => {
                                    println!("opposite");
                                    selection_state.selection = Selection::Section(opposite);
                                }
                                Err(()) => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Sample8);
        app.add_plugins(PanCamPlugin);
        app.add_plugins(MousePosPlugin);
        app.add_plugins(WorldInspectorPlugin::default());
        app.add_plugins(ShapePlugin);
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
                update_hover,
                update_track_color,
                draw_selection,
                extend_selection,
            ),
        );
    }
}
