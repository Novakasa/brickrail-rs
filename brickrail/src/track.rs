use crate::{
    block::{Block, BlockCreateMessage},
    crossing::{LevelCrossing, SpawnCrossingMessage},
    editor::{
        DespawnMessage, EditorState, GenericID, HoverState, MousePosWorld, Selection,
        SelectionState, delete_selection_shortcut, finish_hover,
    },
    inspector::{Inspectable, InspectorPlugin},
    layout::{Connections, EntityMap, TrackLocks},
    layout_primitives::*,
    marker::{Marker, MarkerColor, MarkerSpawnMessage},
    materials::{TrackBaseMaterial, TrackInnerMaterial, TrackPathMaterial},
    route::LegState,
    selectable::{Selectable, SelectablePlugin, SelectableType},
    switch::{Switch, UpdateSwitchTurnsMessage},
    track_mesh::{MeshType, TrackMeshPlugin},
    train::{PlanRouteEvent, Train, TrainDragState},
    utils::bresenham_line,
};
use bevy::{
    color::palettes::css::*, ecs::system::SystemState, math::vec4, platform::collections::HashSet,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_egui::egui::Ui;
use bevy_inspector_egui::bevy_egui;
use bevy_prototype_lyon::prelude::*;
use lyon_tessellation::{
    LineCap, StrokeOptions,
    math::Point,
    path::{BuilderWithAttributes, Path},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub const TRACK_WIDTH: f32 = 10.0;
pub const TRACK_INNER_WIDTH: f32 = 6.0;
pub const PATH_WIDTH: f32 = TRACK_WIDTH * 0.25;
pub const LAYOUT_SCALE: f32 = 40.0;

#[derive(Resource, Default)]
struct TrackBuildState {
    hover_cells: Vec<CellID>,
    hover_track: Option<TrackID>,
    portal_entrance: Option<DirectedTrackID>,
}

pub fn build_connection_path(dirconnection: DirectedTrackConnectionID) -> Path {
    let length = dirconnection.connection_length() * 0.5;
    build_connection_path_extents(dirconnection, 0.0, length)
}

pub fn vec_point(vec: Vec2) -> Point {
    Point::new(vec.x, vec.y)
}

pub fn build_connection_path_extents(
    dirconnection: DirectedTrackConnectionID,
    from: f32,
    to: f32,
) -> Path {
    let mut path_builder = BuilderWithAttributes::new(2);
    path_builder.begin(
        vec_point(dirconnection.interpolate_pos(from) * LAYOUT_SCALE),
        &[from, 0.0],
    );
    let num_segments = match dirconnection.curve_index() {
        0 => 1,
        _ => 5,
    };
    let epsilon = 0.001;
    for i in 0..(num_segments + 1) {
        let dist = from + epsilon + i as f32 * (to - from - epsilon) / num_segments as f32;
        path_builder.line_to(
            vec_point(dirconnection.interpolate_pos(dist) * LAYOUT_SCALE),
            &[dist, dist / (to - from)],
        );
    }
    path_builder.line_to(
        vec_point(dirconnection.interpolate_pos(to) * LAYOUT_SCALE),
        &[to, 1.0],
    );
    path_builder.end(false);

    path_builder.build()
}

impl TrackBuildState {
    fn build(
        &mut self,
        connections: &mut Connections,
        track_event_writer: &mut MessageWriter<SpawnTrackMessage>,
        connection_message_writer: &mut MessageWriter<SpawnConnectionMessage>,
    ) {
        while self.hover_cells.len() > 2 {
            if let Some(track_id) = TrackID::from_cells(
                self.hover_cells[0],
                self.hover_cells[1],
                self.hover_cells[2],
            ) {
                if !connections.has_track(track_id) {
                    track_event_writer.write(SpawnTrackMessage(Track::from_id(track_id)));
                }
                if let Some(track_b) = self.hover_track {
                    if let Some(connection_id) = track_b.get_connection_to(track_id) {
                        if !connections.has_connection(&connection_id) {
                            connection_message_writer.write(SpawnConnectionMessage {
                                id: connection_id,
                                update_switches: true,
                            });
                        }
                    }
                }
                self.hover_track = Some(track_id);
            }
            self.hover_cells.remove(0);
        }
    }
}

pub fn track_section_inspector(ui: &mut Ui, world: &mut World) {
    let mut state = SystemState::<(
        Res<EntityMap>,
        Res<SelectionState>,
        Res<AppTypeRegistry>,
        MessageWriter<BlockCreateMessage>,
    )>::new(world);
    let (_entity_map, selection_state, _type_registry, mut spawn_events) = state.get_mut(world);
    if let Selection::Section(section) = &selection_state.selection {
        ui.label("Section inspector");
        ui.separator();
        ui.label(format!("Tracks: {}", section.len()));
        ui.separator();
        if ui.button("Create block").clicked() {
            let block = Block::new(section.clone());
            spawn_events.write(BlockCreateMessage(block));
        }
        ui.separator();
    }
}

pub fn spawn_track(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    mut entity_map: ResMut<EntityMap>,
    mut event_reader: MessageReader<SpawnTrackMessage>,
) {
    for request in event_reader.read() {
        let track = request.0.clone();
        let track_id = track.id;
        connections.add_filtered_track(track_id, &track.logical_filter);
        let entity = commands.spawn(TrackBundle::from_track(track)).id();
        entity_map.add_track(track_id, entity);
    }
}

#[derive(Debug, Clone, Message)]
pub struct SpawnConnectionMessage {
    pub id: TrackConnectionID,
    pub update_switches: bool,
}

impl Serialize for SpawnConnectionMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.id.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SpawnConnectionMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            id: TrackConnectionID::deserialize(deserializer)?,
            update_switches: false,
        })
    }
}

pub fn spawn_connection(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    mut entity_map: ResMut<EntityMap>,
    mut event_reader: MessageReader<SpawnConnectionMessage>,
    mut switch_update_events: MessageWriter<UpdateSwitchTurnsMessage>,
    mut base_materials: ResMut<Assets<TrackBaseMaterial>>,
    mut inner_materials: ResMut<Assets<TrackInnerMaterial>>,
    mut path_materials: ResMut<Assets<TrackPathMaterial>>,
) {
    for spawn_connection in event_reader.read() {
        let connection_id = spawn_connection.id;
        for directed in connection_id.directed_connections() {
            let base_material = MeshMaterial2d(base_materials.add(TrackBaseMaterial {
                color: LinearRgba::from(WHITE),
            }));
            let outer_entity = commands
                .spawn((TrackShapeOuter::new(directed), base_material))
                .id();
            let inner_material = MeshMaterial2d(inner_materials.add(TrackInnerMaterial {
                color: LinearRgba::from(BLACK),
            }));
            let inner_entity = commands
                .spawn((TrackShapeInner::new(directed), inner_material))
                .id();
            let path_material = MeshMaterial2d(path_materials.add(TrackPathMaterial {
                color: BLACK.with_alpha(0.0).into(),
                direction: 0,
            }));
            let path_entity = commands
                .spawn((TrackShapePath::new(directed), path_material))
                .id();
            connections.connect_tracks_simple(&connection_id);
            entity_map.add_connection(directed, outer_entity, inner_entity, path_entity);
        }

        if spawn_connection.update_switches {
            for track_id in connection_id.tracks() {
                let existing_connections = connections.get_directed_connections_from(track_id);
                let event = UpdateSwitchTurnsMessage {
                    id: track_id,
                    positions: existing_connections
                        .iter()
                        .map(|c| c.get_switch_position())
                        .collect::<Vec<SwitchPosition>>(),
                };
                println!("{:?}", event);
                switch_update_events.write(event);
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum TrackShapeType {
    Outer,
    Inner,
    Path,
}

#[derive(Debug, Component)]
struct TrackShapeOuter {
    id: DirectedTrackConnectionID,
}

impl TrackShapeOuter {
    pub fn new(id: DirectedTrackConnectionID) -> Self {
        Self { id }
    }
}

impl MeshType for TrackShapeOuter {
    type ID = DirectedConnectionShape;

    fn id(&self) -> Self::ID {
        self.id.shape_id()
    }

    fn stroke() -> StrokeOptions {
        StrokeOptions::default()
            .with_line_width(TRACK_WIDTH)
            .with_line_cap(LineCap::Round)
    }

    fn base_transform(&self) -> Transform {
        Transform::from_translation(
            (self.id.from_track.cell().get_vec2() * LAYOUT_SCALE).extend(1.0),
        )
    }

    fn path(&self) -> Path {
        build_connection_path(self.id().to_connection(CellID::new(0, 0, 0)))
    }

    fn interpolate(&self, dist: f32) -> Vec2 {
        self.id()
            .to_connection(CellID::new(0, 0, 0))
            .interpolate_pos(dist)
    }

    fn build_mesh(&self) -> Mesh {
        let mut mesh = self.build_path_mesh();
        if self.id().is_portal {
            let mut circle = Circle::new(TRACK_WIDTH * 0.7).mesh().build().translated_by(
                (self.id.from_track.to_slot().get_vec2() - self.id.from_track.cell().get_vec2())
                    .extend(0.0)
                    * LAYOUT_SCALE,
            );
            circle.insert_attribute(
                Mesh::ATTRIBUTE_COLOR,
                vec![vec4(1.0, 1.0, 1.0, 1.0); circle.get_vertex_size() as usize],
            );
            mesh.merge(&circle).unwrap();
        }
        mesh
    }
}

#[derive(Debug, Component)]
struct TrackShapeInner {
    id: DirectedTrackConnectionID,
}

impl TrackShapeInner {
    pub fn new(id: DirectedTrackConnectionID) -> Self {
        Self { id }
    }
}

impl MeshType for TrackShapeInner {
    type ID = DirectedConnectionShape;

    fn id(&self) -> Self::ID {
        self.id.shape_id()
    }

    fn base_transform(&self) -> Transform {
        Transform::from_translation(
            (self.id.from_track.cell().get_vec2() * LAYOUT_SCALE).extend(2.0),
        )
    }

    fn stroke() -> StrokeOptions {
        StrokeOptions::default()
            .with_line_width(TRACK_INNER_WIDTH)
            .with_line_cap(LineCap::Round)
    }

    fn path(&self) -> Path {
        build_connection_path(self.id().to_connection(CellID::new(0, 0, 0)))
    }

    fn interpolate(&self, dist: f32) -> Vec2 {
        self.id()
            .to_connection(CellID::new(0, 0, 0))
            .interpolate_pos(dist)
    }
}

#[derive(Debug, Component)]
struct TrackShapePath {
    id: DirectedTrackConnectionID,
}

impl TrackShapePath {
    pub fn new(id: DirectedTrackConnectionID) -> Self {
        Self { id }
    }
}

impl MeshType for TrackShapePath {
    type ID = DirectedConnectionShape;

    fn id(&self) -> Self::ID {
        self.id.shape_id()
    }

    fn base_transform(&self) -> Transform {
        Transform::from_translation(
            (self.id.from_track.cell().get_vec2() * LAYOUT_SCALE).extend(19.0),
        )
    }

    fn stroke() -> StrokeOptions {
        StrokeOptions::default()
            .with_line_width(PATH_WIDTH)
            .with_line_cap(LineCap::Butt)
    }

    fn path(&self) -> Path {
        build_connection_path(self.id().to_connection(CellID::new(0, 0, 0)))
    }

    fn interpolate(&self, dist: f32) -> Vec2 {
        self.id()
            .to_connection(CellID::new(0, 0, 0))
            .interpolate_pos(dist)
    }
}

#[derive(Debug, Clone)]
pub struct TrackLogicalFilter {
    pub filters: HashMap<LogicalDiscriminator, bool>,
}

impl TrackLogicalFilter {
    pub fn default() -> Self {
        let mut filters = HashMap::new();
        for facing in [Facing::Forward, Facing::Backward] {
            for direction in [TrackDirection::First, TrackDirection::Last] {
                filters.insert(LogicalDiscriminator { direction, facing }, true);
            }
        }
        Self { filters }
    }

    pub fn is_default(&self) -> bool {
        // false if any entry is false
        self.filters.iter().all(|(_, value)| *value)
    }
}

impl Serialize for TrackLogicalFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let filtered_keys: Vec<LogicalDiscriminator> = self
            .filters
            .iter()
            .filter(|(_, value)| !**value)
            .map(|(key, _)| key.clone())
            .collect();
        filtered_keys.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TrackLogicalFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut filter = Self::default();
        let filtered_keys: Vec<LogicalDiscriminator> = Vec::deserialize(deserializer)?;
        for key in filtered_keys {
            filter.filters.insert(key, false);
        }
        Ok(filter)
    }
}

#[derive(Debug, Clone, Message)]
pub struct SpawnTrackMessage(pub Track);

impl Serialize for SpawnTrackMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.logical_filter.is_default() {
            self.0.id.serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for SpawnTrackMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let track = match serde_json::from_value::<TrackID>(value.clone()) {
            Ok(id) => Track::from_id(id),
            Err(_) => serde_json::from_value::<Track>(value).unwrap(),
        };
        Ok(SpawnTrackMessage(track))
    }
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackID,
    pub logical_filter: TrackLogicalFilter,
}

impl Track {
    pub fn from_id(id: TrackID) -> Self {
        Self {
            id,
            logical_filter: TrackLogicalFilter::default(),
        }
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Track>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            MessageWriter<MarkerSpawnMessage>,
            MessageWriter<SpawnCrossingMessage>,
            ResMut<Connections>,
            ResMut<TrackBuildState>,
            MessageWriter<SpawnConnectionMessage>,
        )>::new(world);
        let (
            mut tracks,
            entity_map,
            selection_state,
            _type_registry,
            mut marker_spawner,
            mut crossing_spawner,
            mut connections,
            mut track_build_state,
            mut connection_spawner,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut track) = tracks.get_mut(entity) {
                ui.label("Inspectable track lol");
                if !entity_map.markers.contains_key(&track.id) {
                    if ui.button("Add Marker").clicked() {
                        let id = track.id.clone();

                        let marker = Marker::new(id, MarkerColor::Red);
                        marker_spawner.write(MarkerSpawnMessage(marker));
                    }
                }
                if !entity_map.crossings.contains_key(&track.id) {
                    if ui.button("Add Crossing").clicked() {
                        let id = track.id.clone();
                        let crossing = LevelCrossing::new(id);
                        crossing_spawner.write(SpawnCrossingMessage::new(crossing));
                    }
                }
                ui.separator();
                ui.heading("Logical filters");
                let track_id = track.id;
                let mut changed = false;
                for (logical, value) in track.logical_filter.filters.iter_mut() {
                    let logical_track = track_id
                        .get_directed(logical.direction)
                        .get_logical(logical.facing);
                    ui.push_id(logical, |ui| {
                        changed |= ui
                            .checkbox(value, format!("{}", logical_track.get_dirstring()))
                            .changed();
                    });
                }
                if changed {
                    println!("Changed logical filters");
                    connections.add_filtered_track(track_id, &track.logical_filter)
                }
                ui.separator();
                match track_build_state.portal_entrance {
                    None => {
                        if let Some(directed) = connections.get_unconnected_dirtrack(track_id) {
                            if ui.button("Set as portal entrance").clicked() {
                                track_build_state.portal_entrance = Some(directed);
                            }
                        }
                    }
                    Some(entrance) => {
                        if let Some(directed) = connections.get_unconnected_dirtrack(track_id) {
                            if directed != entrance {
                                if ui.button("Set as portal exit").clicked() {
                                    let connection_id = TrackConnectionID::new(entrance, directed);
                                    track_build_state.portal_entrance = None;
                                    connection_spawner.write(SpawnConnectionMessage {
                                        id: connection_id,
                                        update_switches: true,
                                    });
                                }
                            } else {
                                ui.label("Select exit track to create portal");
                            }
                        }
                        if ui.button("Clear portal entrance").clicked() {
                            track_build_state.portal_entrance = None;
                        }
                    }
                }
                ui.separator();
            }
        }
    }
}

impl Inspectable for Track {
    fn inspector(ui: &mut Ui, world: &mut World) {
        Track::inspector(ui, world);
    }

    fn run_condition(selection_state: Res<SelectionState>) -> bool {
        selection_state.selected_type() == Some(SelectableType::Track)
    }
}

impl Selectable for Track {
    type SpawnMessage = SpawnTrackMessage;
    type ID = TrackID;

    fn get_type() -> crate::selectable::SelectableType {
        crate::selectable::SelectableType::Track
    }

    fn get_depth(&self) -> f32 {
        1.0
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Track(self.id)
    }

    fn id(&self) -> Self::ID {
        self.id
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Shape>,
    ) -> f32 {
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
            track: Track::from_id(track_id),
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
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
        let first_cell = CellID::from_vec2(mouse_world_pos.pos / LAYOUT_SCALE);
        track_build_state.hover_cells.push(first_cell);
    }
}

fn exit_draw_track(
    mut track_build_state: ResMut<TrackBuildState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
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
    mut track_event_writer: MessageWriter<SpawnTrackMessage>,
    mut connection_message_writer: MessageWriter<SpawnConnectionMessage>,
) {
    let last_cell = track_build_state.hover_cells.last();
    if last_cell.is_none() {
        return;
    }
    let start = (last_cell.unwrap().x, last_cell.unwrap().y);
    let mouse_cell = CellID::from_vec2(mouse_world_pos.pos / LAYOUT_SCALE);
    for point in bresenham_line(start, (mouse_cell.x, mouse_cell.y)).iter() {
        let cell = CellID::new(point.0, point.1, 0);
        track_build_state.hover_cells.push(cell);
        // println!("{:?}", track_build_state.hover_cells);
        track_build_state.build(
            &mut connections,
            &mut track_event_writer,
            &mut connection_message_writer,
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
            Color::from(GRAY),
        );
    }
    let cell = CellID::from_vec2(mouse_world_pos.pos / LAYOUT_SCALE);
    gizmos.circle_2d(
        cell.get_vec2() * LAYOUT_SCALE,
        LAYOUT_SCALE * 0.25,
        Color::from(RED),
    );

    let scale = LAYOUT_SCALE;

    if let Some(track) = track_build_state.hover_track {
        for dirtrack in track.dirtracks() {
            dirtrack.draw_with_gizmos(&mut gizmos, scale, Color::from(RED))
        }
    }
}

fn update_path_track(
    _trigger: On<PlanRouteEvent>,
    mut query: Query<(
        &TrackShapePath,
        &mut Transform,
        &MeshMaterial2d<TrackPathMaterial>,
    )>,
    track_locks: Res<TrackLocks>,
    trains: Query<&Train>,
    drag_train: Res<TrainDragState>,
    mut path_materials: ResMut<Assets<TrackPathMaterial>>,
) {
    let mut route_connections = HashSet::new();
    for train in trains.iter() {
        for leg in train.get_route().iter_legs_remaining() {
            let section = match leg.get_leg_state() {
                LegState::Completed => &leg.to_section,
                _ => &leg.travel_section,
            };
            for connection in section.directed_connection_iter() {
                route_connections.insert(connection);
            }
        }
    }
    if let Some(route) = drag_train.route.as_ref() {
        for leg in route.iter_legs_remaining() {
            for connection in leg.travel_section.directed_connection_iter() {
                route_connections.insert(connection);
            }
        }
    }

    for (connection, mut transform, material_handle) in query.iter_mut() {
        let z = connection.base_transform().translation.z;
        let dir = match (
            route_connections.contains(&connection.id),
            route_connections.contains(&connection.id.opposite()),
        ) {
            (true, false) => 1,
            (false, true) => -1,
            (true, true) => 2,
            (false, false) => 0,
        };

        let material = path_materials.get_mut(material_handle).unwrap();

        match dir {
            0 => {
                material.color = BLACK.with_alpha(0.0).into();
                transform.translation.z = z;
                material.direction = 0;
            }
            _ => {
                if track_locks
                    .locked_tracks
                    .contains_key(&connection.id.from_track.track)
                {
                    material.color = LinearRgba::from(ORANGE);
                    transform.translation.z = z + 0.5;
                } else {
                    material.color = LinearRgba::from(BLUE);
                    transform.translation.z = z + 0.3;
                    debug!("blue");
                }
                material.direction = dir;
                debug!("Setting direction to {}", dir);
            }
        }
    }
}

fn update_inner_track(
    mut q_strokes: Query<(
        &TrackShapeInner,
        &mut Transform,
        &MeshMaterial2d<TrackInnerMaterial>,
    )>,
    hover_state: Res<HoverState>,
    selection_state: Res<SelectionState>,
    mut inner_materials: ResMut<Assets<TrackInnerMaterial>>,
) {
    if !selection_state.is_changed() && !hover_state.is_changed() {
        return;
    }
    for (connection, mut transform, material_handle) in q_strokes.iter_mut() {
        let z = connection.base_transform().translation.z;
        if hover_state.hover == Some(GenericID::Track(connection.id.from_track.track)) {
            inner_materials.get_mut(material_handle).unwrap().color = LinearRgba::from(RED);
            transform.translation.z = z + 0.5;
            continue;
        }

        if selection_state.selection
            == Selection::Single(GenericID::Track(connection.id.from_track.track))
        {
            inner_materials.get_mut(material_handle).unwrap().color = LinearRgba::from(BLUE);
            transform.translation.z = z + 0.3;
            continue;
        }

        if let Selection::Section(section) = &selection_state.selection {
            if section.has_track(&connection.id.from_track.track) {
                inner_materials.get_mut(material_handle).unwrap().color = LinearRgba::from(BLUE);
                transform.translation.z = z + 0.3;
                continue;
            }
        }

        inner_materials.get_mut(material_handle).unwrap().color = LinearRgba::from(BLACK);
        transform.translation.z = z;
    }
}

fn despawn_track(
    mut commands: Commands,
    mut connections: ResMut<Connections>,
    mut entity_map: ResMut<EntityMap>,
    mut event_reader: MessageReader<DespawnMessage<Track>>,
    mut switch_update_events: MessageWriter<UpdateSwitchTurnsMessage>,
    mut switch_despawn_events: MessageWriter<DespawnMessage<Switch>>,
) {
    for despawn_event in event_reader.read() {
        let track_id = despawn_event.0;

        for switch in track_id
            .dirtracks()
            .iter()
            .filter(|id| entity_map.switches.contains_key(*id))
        {
            switch_despawn_events.write(DespawnMessage(*switch));
        }

        let mut other_dirtracks = vec![];

        for (_, _, connection) in connections.connection_graph.edges(track_id) {
            for directed in connection.directed_connections() {
                let outer = entity_map.connections_outer.get(&directed).unwrap().clone();
                commands.entity(outer).despawn();
                let inner = entity_map.connections_inner.get(&directed).unwrap().clone();
                commands.entity(inner).despawn();
                let path = entity_map.connections_path.get(&directed).unwrap().clone();
                commands.entity(path).despawn();
                entity_map.remove_connection(directed);
                for other in connection.tracks() {
                    if other.track != track_id {
                        other_dirtracks.push(other);
                    }
                }
            }
        }

        let entity = entity_map.tracks.get(&track_id).unwrap().clone();
        commands.entity(entity).despawn();
        connections.remove_track(track_id);
        entity_map.remove_track(track_id);

        for directed in other_dirtracks {
            let existing_connections = connections.get_directed_connections_from(directed);
            let event = UpdateSwitchTurnsMessage {
                id: directed,
                positions: existing_connections
                    .iter()
                    .map(|c| c.get_switch_position())
                    .collect::<Vec<SwitchPosition>>(),
            };
            println!("{:?}", event);
            switch_update_events.write(event);
        }
    }
}

struct TrackSectionSelection;

impl Inspectable for TrackSectionSelection {
    fn inspector(ui: &mut Ui, world: &mut World) {
        track_section_inspector(ui, world);
    }

    fn run_condition(selection_state: Res<SelectionState>) -> bool {
        matches!(selection_state.selection, Selection::Section(_))
    }
}

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackBuildState::default());
        app.add_plugins(TrackMeshPlugin::<TrackShapeOuter>::default());
        app.add_plugins(TrackMeshPlugin::<TrackShapeInner>::default());
        app.add_plugins(TrackMeshPlugin::<TrackShapePath>::default());
        app.add_plugins(SelectablePlugin::<Track>::new());
        app.add_plugins(InspectorPlugin::<Track>::new());
        app.add_plugins(InspectorPlugin::<TrackSectionSelection>::new());
        app.add_message::<SpawnTrackMessage>();
        app.add_message::<SpawnConnectionMessage>();
        app.add_message::<DespawnMessage<Track>>();
        app.add_observer(update_path_track);
        app.add_systems(
            Update,
            (
                init_draw_track.run_if(in_state(EditorState::Edit)),
                exit_draw_track.run_if(in_state(EditorState::Edit)),
                update_draw_track.run_if(in_state(EditorState::Edit)),
                update_inner_track.after(finish_hover),
                draw_build_cells.run_if(in_state(EditorState::Edit)),
                delete_selection_shortcut::<Track>.run_if(in_state(EditorState::Edit)),
                despawn_track,
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                spawn_track.run_if(on_message::<SpawnTrackMessage>),
                spawn_connection
                    .run_if(on_message::<SpawnConnectionMessage>)
                    .after(spawn_track),
            ),
        );
    }
}
