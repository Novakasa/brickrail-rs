use bevy::color::palettes::css::{BLUE, GREEN, RED, YELLOW};
use bevy::ecs::system::SystemState;
use bevy::platform::collections::HashMap;
use bevy::{gizmos::gizmos::Gizmos, prelude::*, reflect::Reflect};
use bevy_egui::egui::Ui;
use bevy_inspector_egui::bevy_egui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

use crate::selectable::{Selectable, SelectablePlugin};
use crate::{
    editor::*,
    layout::{EntityMap, MarkerMap},
    layout_primitives::*,
    track::{spawn_track, LAYOUT_SCALE},
};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect)]
pub enum MarkerKey {
    Enter,
    In,
    None,
}

impl MarkerKey {
    pub fn as_train_u8(&self) -> u8 {
        match self {
            MarkerKey::Enter => 1,
            MarkerKey::In => 2,
            MarkerKey::None => 0,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Default,
    Serialize,
    Deserialize,
    Reflect,
)]
pub enum MarkerSpeed {
    Slow,
    #[default]
    Cruise,
    Fast,
}

impl MarkerSpeed {
    pub fn get_speed(&self) -> f32 {
        match self {
            MarkerSpeed::Slow => 2.0,
            MarkerSpeed::Cruise => 4.0,
            MarkerSpeed::Fast => 8.0,
        }
    }

    pub fn as_train_u8(&self) -> u8 {
        match self {
            MarkerSpeed::Slow => 2,
            MarkerSpeed::Cruise => 3,
            MarkerSpeed::Fast => 1,
        }
    }
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Serialize, Deserialize, Reflect,
)]
pub enum MarkerColor {
    Any,
    Red,
    Blue,
    Yellow,
    Green,
}

impl MarkerColor {
    pub fn as_train_u8(&self) -> u8 {
        match self {
            MarkerColor::Any => 15,
            MarkerColor::Red => 3,
            MarkerColor::Blue => 1,
            MarkerColor::Yellow => 0,
            MarkerColor::Green => 2,
        }
    }

    pub fn from_train_u8(value: u8) -> Option<Self> {
        let color = match value {
            15 => MarkerColor::Any,
            3 => MarkerColor::Red,
            1 => MarkerColor::Blue,
            0 => MarkerColor::Yellow,
            2 => MarkerColor::Green,
            _ => {
                return None;
            }
        };
        Some(color)
    }

    pub fn get_display_color(&self) -> Color {
        match self {
            MarkerColor::Any => Color::WHITE,
            MarkerColor::Red => Color::from(RED),
            MarkerColor::Blue => Color::from(BLUE),
            MarkerColor::Yellow => Color::from(YELLOW),
            MarkerColor::Green => Color::from(GREEN),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct LogicalMarkerData {
    pub speed: MarkerSpeed,
}

#[derive(Debug, Component, Serialize, Deserialize, Clone, Reflect)]
pub struct Marker {
    pub track: TrackID,
    pub color: MarkerColor,
    #[serde(with = "any_key_map")]
    pub logical_data: HashMap<LogicalTrackID, LogicalMarkerData>,
}

impl Marker {
    pub fn new(track: TrackID, color: MarkerColor) -> Self {
        let mut logical_data = HashMap::new();
        for logical in track.logical_tracks() {
            logical_data.insert(logical, LogicalMarkerData::default());
        }
        Self {
            track: track,
            color: color,
            logical_data: logical_data,
        }
    }

    pub fn get_logical_data(&self, logical: LogicalTrackID) -> Option<&LogicalMarkerData> {
        self.logical_data.get(&logical)
    }

    pub fn set_logical_data(&mut self, logical: LogicalTrackID, data: LogicalMarkerData) {
        self.logical_data.insert(logical, data);
    }

    pub fn draw_with_gizmos(&self, gizmos: &mut Gizmos) {
        let position = self
            .track
            .get_directed(TrackDirection::First)
            .get_center_vec2()
            * LAYOUT_SCALE;
        gizmos.circle_2d(position, 0.05 * LAYOUT_SCALE, Color::WHITE);
        gizmos.circle_2d(
            position,
            0.02 * LAYOUT_SCALE,
            self.color.get_display_color(),
        );
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Marker>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
        )>::new(world);
        let (mut markers, entity_map, selection_state, type_registry) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut marker) = markers.get_mut(entity) {
                ui.label("Inspectable marker lol");
                ui_for_value(&mut marker.color, ui, &type_registry.read());
                ui.label("Logical data");
                for (logical, data) in marker.logical_data.iter_mut() {
                    ui.push_id(logical, |ui| {
                        ui.label(logical.get_dirstring());
                        ui_for_value(data, ui, &type_registry.read());
                    });
                }
                ui.separator();
            }
        }
    }
}

impl Selectable for Marker {
    type SpawnEvent = MarkerSpawnEvent;
    type ID = TrackID;

    fn inspector(ui: &mut Ui, world: &mut World) {
        Marker::inspector(ui, world);
    }

    fn get_type() -> crate::selectable::SelectableType {
        crate::selectable::SelectableType::Marker
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Marker(self.track)
    }

    fn id(&self) -> Self::ID {
        self.track
    }

    fn get_depth(&self) -> f32 {
        2.0
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Shape>,
    ) -> f32 {
        self.track
            .get_directed(TrackDirection::First)
            .get_center_vec2()
            .distance(pos)
            - 0.05
    }
}

#[derive(Debug, Event, Clone, Serialize, Deserialize)]
pub struct MarkerSpawnEvent(pub Marker);

fn create_marker(
    selection_state: Res<SelectionState>,
    mut marker_events: EventWriter<MarkerSpawnEvent>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        if let Selection::Single(GenericID::Track(track_id)) = selection_state.selection {
            let marker = Marker::new(track_id, MarkerColor::Any);
            marker_events.write(MarkerSpawnEvent(marker));
        }
    }
}

pub fn spawn_marker(
    mut commands: Commands,
    mut marker_events: EventReader<MarkerSpawnEvent>,
    mut entity_map: ResMut<EntityMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in marker_events.read() {
        let marker = event.0.clone();
        let track_id = marker.track;
        let mesh = Circle::new(0.05 * LAYOUT_SCALE).mesh().build();
        let material = ColorMaterial::from(marker.color.get_display_color());
        let transform = Transform::from_translation(
            (marker
                .track
                .get_directed(TrackDirection::First)
                .get_center_vec2()
                * LAYOUT_SCALE)
                .extend(25.0),
        );
        let entity = commands
            .spawn((
                Mesh2d(meshes.add(mesh).into()),
                transform,
                MeshMaterial2d(materials.add(material)),
                marker,
            ))
            .id();
        entity_map.add_marker(track_id, entity);
    }
}

fn set_marker_color(
    markers: Query<(&Marker, &MeshMaterial2d<ColorMaterial>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    hover_state: Res<HoverState>,
    selection_state: Res<SelectionState>,
) {
    for (marker, material) in markers.iter() {
        let mut color = marker.color.get_display_color();
        if selection_state.selection == Selection::Single(GenericID::Marker(marker.track)) {
            color = Color::from(RED);
        }
        if hover_state.hover == Some(GenericID::Marker(marker.track)) {
            color = Color::from(BLUE);
        }
        let material = materials.get_mut(material).unwrap();
        material.color = color;
    }
}

pub fn despawn_marker(
    mut commands: Commands,
    mut marker_events: EventReader<DespawnEvent<Marker>>,
    mut entity_map: ResMut<EntityMap>,
    mut marker_map: ResMut<MarkerMap>,
) {
    for event in marker_events.read() {
        let track_id = event.0;
        let entity = entity_map.markers.get(&track_id).unwrap().clone();
        commands.entity(entity.clone()).despawn();
        entity_map.remove_marker(track_id);
        marker_map.remove_marker(track_id);
    }
}

fn draw_markers(q_markers: Query<&Marker>, mut gizmos: Gizmos) {
    for marker in q_markers.iter() {
        marker.draw_with_gizmos(&mut gizmos);
    }
}

pub struct MarkerPlugin;

impl Plugin for MarkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SelectablePlugin::<Marker>::new());
        app.add_event::<MarkerSpawnEvent>();
        app.add_event::<DespawnEvent<Marker>>();
        app.add_systems(
            Update,
            (
                create_marker,
                delete_selection_shortcut::<Marker>,
                set_marker_color.after(finish_hover),
            ),
        );
        app.add_systems(
            PostUpdate,
            (
                spawn_marker
                    .run_if(on_event::<MarkerSpawnEvent>)
                    .after(spawn_track),
                despawn_marker,
            ),
        );
    }
}
