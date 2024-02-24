use crate::{
    editor::{GenericID, HoverState, Selectable, Selection, SelectionState, SpawnEvent},
    layout::{Connections, EntityMap},
    layout_primitives::*,
    marker::{Marker, MarkerColor},
    utils::bresenham_line,
};
use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::egui::Ui;
use bevy_mouse_tracking_plugin::MousePosWorld;
use bevy_prototype_lyon::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

pub const TRACK_WIDTH: f32 = 10.0;
pub const TRACK_INNER_WIDTH: f32 = 6.0;
pub const LAYOUT_SCALE: f32 = 40.0;

#[derive(Resource, Default)]
struct TrackBuildState {
    hover_cells: Vec<CellID>,
    hover_track: Option<TrackID>,
}

fn build_connection_path(connection: TrackConnectionID) -> Path {
    let dirconnection = connection.to_directed(ConnectionDirection::Aligned);
    let mut path_builder = PathBuilder::new();
    let length = dirconnection.connection_length();
    path_builder.move_to(dirconnection.interpolate_pos(0.0) * LAYOUT_SCALE);
    let num_segments = match dirconnection.curve_index() {
        0 => 1,
        _ => 10,
    };
    for i in 1..(num_segments + 1) {
        let dist = i as f32 * length / num_segments as f32;
        path_builder.line_to(dirconnection.interpolate_pos(dist) * LAYOUT_SCALE);
    }

    path_builder.build()
}

impl TrackBuildState {
    fn build(
        &mut self,
        connections: &mut Connections,
        track_event_writer: &mut EventWriter<SpawnEvent<Track>>,
        connection_event_writer: &mut EventWriter<SpawnEvent<TrackConnection>>,
    ) {
        while self.hover_cells.len() > 2 {
            if let Some(track_id) = TrackID::from_cells(
                self.hover_cells[0],
                self.hover_cells[1],
                self.hover_cells[2],
            ) {
                if !connections.has_track(track_id) {
                    track_event_writer.send(SpawnEvent(Track { id: track_id }));
                }
                if let Some(track_b) = self.hover_track {
                    if let Some(connection_id) = track_b.get_connection_to(track_id) {
                        if !connections.has_connection(&connection_id) {
                            connection_event_writer
                                .send(SpawnEvent(TrackConnection::new(connection_id)));
                        }
                    }
                }
                self.hover_track = Some(track_id);
            }
            self.hover_cells.remove(0);
        }
    }
}

pub fn spawn_track(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    mut entity_map: ResMut<EntityMap>,
    mut event_reader: EventReader<SpawnEvent<Track>>,
) {
    for request in event_reader.read() {
        let track = request.0.clone();
        let track_id = track.id;
        connections.add_track(track_id);
        let entity = commands.spawn(TrackBundle::from_track(track)).id();
        entity_map.add_track(track_id, entity);
    }
}

fn spawn_connection(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    mut entity_map: ResMut<EntityMap>,
    mut event_reader: EventReader<SpawnEvent<TrackConnection>>,
) {
    for request in event_reader.read() {
        let connection = request.0.clone();
        let connection_id = connection.id;
        let entity = commands.spawn(TrackConnection::new(connection_id)).id();
        let outer_entity = commands
            .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Outer))
            .id();
        let inner_entity = commands
            .spawn(TrackBaseShape::new(connection_id, TrackShapeType::Inner))
            .id();
        connections.connect_tracks_simple(&connection_id);
        entity_map.add_connection(connection_id, entity, outer_entity, inner_entity);
    }
}

#[derive(PartialEq, Eq)]
pub enum TrackShapeType {
    Outer,
    Inner,
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct TrackConnection {
    pub id: TrackConnectionID,
}

impl TrackConnection {
    pub fn new(id: TrackConnectionID) -> Self {
        Self { id: id }
    }
}

impl Selectable for TrackConnection {
    fn get_id(&self) -> GenericID {
        GenericID::TrackConnection(self.id)
    }
}

#[derive(Component)]
pub struct TrackConnectionShape {
    id: TrackConnectionID,
    shape_type: TrackShapeType,
}

#[derive(Bundle)]
pub struct TrackBaseShape {
    connection: TrackConnectionShape,
    shape: ShapeBundle,
    stroke: Stroke,
}

impl TrackBaseShape {
    pub fn new(id: TrackConnectionID, shape_type: TrackShapeType) -> Self {
        let (color, width, z) = match &shape_type {
            TrackShapeType::Inner => (Color::BLACK, TRACK_INNER_WIDTH, 10.0),
            TrackShapeType::Outer => (Color::WHITE, TRACK_WIDTH, 5.0),
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

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct Track {
    pub id: TrackID,
}

impl Track {
    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Track>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            EventWriter<SpawnEvent<Marker>>,
        )>::new(world);
        let (mut tracks, entity_map, selection_state, _type_registry, mut marker_spawner) =
            state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(track) = tracks.get_mut(entity) {
                ui.label("Inspectable track lol");
                if !entity_map.markers.contains_key(&track.id) {
                    if ui.button("Add Marker").clicked() {
                        let id = track.id.clone();

                        let marker = Marker::new(id, MarkerColor::Red);
                        marker_spawner.send(SpawnEvent(marker));
                    }
                }
            }
        }
    }
}

impl Selectable for Track {
    fn get_depth(&self) -> f32 {
        1.0
    }

    fn get_id(&self) -> GenericID {
        GenericID::Track(self.id)
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.id.distance_to(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Bundle)]
pub struct TrackBundle {
    track: Track,
    name: Name,
}

impl TrackBundle {
    pub fn new(track_id: TrackID) -> Self {
        Self {
            track: Track { id: track_id },
            name: Name::new(format!("{:?}", track_id)),
        }
    }
    pub fn from_track(track: Track) -> Self {
        Self {
            name: Name::new(format!("{:?}", track.id)),
            track: track,
        }
    }
}

fn init_draw_track(
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_buttons: Res<Input<MouseButton>>,
    mouse_world_pos: Res<MousePosWorld>,
    hover_state: Res<HoverState>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        match hover_state.hover {
            Some(GenericID::Track(track_id)) => {
                track_build_state.hover_track = Some(track_id);
            }
            None => {
                track_build_state.hover_track = None;
            }
            _ => {
                return;
            }
        }
        let first_cell = CellID::from_vec2(mouse_world_pos.truncate() / LAYOUT_SCALE);
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
    mut connections: ResMut<Connections>,
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_world_pos: Res<MousePosWorld>,
    mut track_event_writer: EventWriter<SpawnEvent<Track>>,
    mut connection_event_writer: EventWriter<SpawnEvent<TrackConnection>>,
) {
    let last_cell = track_build_state.hover_cells.last();
    if last_cell.is_none() {
        return;
    }
    let start = (last_cell.unwrap().x, last_cell.unwrap().y);
    let mouse_cell = CellID::from_vec2(mouse_world_pos.truncate() / LAYOUT_SCALE);
    for point in bresenham_line(start, (mouse_cell.x, mouse_cell.y)).iter() {
        let cell = CellID::new(point.0, point.1, 0);
        track_build_state.hover_cells.push(cell);
        // println!("{:?}", track_build_state.hover_cells);
        track_build_state.build(
            &mut connections,
            &mut track_event_writer,
            &mut connection_event_writer,
        );
    }
}

fn draw_build_cells(
    track_build_state: Res<TrackBuildState>,
    mut gizmos: Gizmos,
    mouse_world_pos: Res<MousePosWorld>,
) {
    for cell in track_build_state.hover_cells.iter() {
        gizmos.circle_2d(
            cell.get_vec2() * LAYOUT_SCALE,
            LAYOUT_SCALE * 0.25,
            Color::GRAY,
        );
    }
    let cell = CellID::from_vec2(mouse_world_pos.truncate() / LAYOUT_SCALE);
    gizmos.circle_2d(
        cell.get_vec2() * LAYOUT_SCALE,
        LAYOUT_SCALE * 0.25,
        Color::RED,
    );

    let scale = LAYOUT_SCALE;

    if let Some(track) = track_build_state.hover_track {
        for dirtrack in track.dirtracks() {
            dirtrack.draw_with_gizmos(&mut gizmos, scale, Color::RED)
        }
    }
}

fn update_track_color(
    mut q_strokes: Query<(&TrackConnectionShape, &mut Stroke, &mut Transform)>,
    hover_state: Res<HoverState>,
    selection_state: Res<SelectionState>,
) {
    if !selection_state.is_changed() && !hover_state.is_changed() {
        return;
    }
    for (connection, mut stroke, mut transform) in q_strokes.iter_mut() {
        if connection.shape_type == TrackShapeType::Outer {
            continue;
        }
        if hover_state.hover == Some(GenericID::Track(connection.id.track_a().track))
            || hover_state.hover == Some(GenericID::Track(connection.id.track_b().track))
        {
            stroke.color = Color::RED;
            transform.translation = Vec3::new(0.0, 0.0, 20.0);
            continue;
        }

        if let Selection::Section(section) = &selection_state.selection {
            if section.has_connection(&connection.id) {
                stroke.color = Color::BLUE;
                transform.translation = Vec3::new(0.0, 0.0, 15.0);
                continue;
            }
        }

        stroke.color = Color::BLACK;
        transform.translation = Vec3::new(0.0, 0.0, 10.0);
    }
}

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackBuildState::default());
        app.register_component_as::<dyn Selectable, Track>();
        app.register_component_as::<dyn Selectable, TrackConnection>();
        app.add_event::<SpawnEvent<Track>>();
        app.add_event::<SpawnEvent<TrackConnection>>();
        app.add_systems(
            Update,
            (
                init_draw_track,
                exit_draw_track,
                update_draw_track,
                update_track_color,
                draw_build_cells,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                spawn_track.run_if(on_event::<SpawnEvent<Track>>()),
                spawn_connection.run_if(on_event::<SpawnEvent<TrackConnection>>()),
            ),
        );
    }
}
