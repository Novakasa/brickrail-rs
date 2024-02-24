use bevy::{
    gizmos::gizmos::Gizmos, prelude::*, reflect::Reflect, render::color::Color, utils::HashMap,
};
use bevy_ecs::system::SystemState;
use bevy_egui::egui::Ui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

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
                        ui.label(&format!("{:?}", logical));
                        ui_for_value(data, ui, &type_registry.read());
                    });
                }
            }
        }
    }
}

impl Selectable for Marker {
    fn get_id(&self) -> GenericID {
        GenericID::Marker(self.track)
    }

    fn get_depth(&self) -> f32 {
        2.0
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.track
            .get_directed(TrackDirection::First)
            .get_center_vec2()
            .distance(pos)
            - 0.05
    }
}

fn create_marker(
    selection_state: Res<SelectionState>,
    mut marker_events: EventWriter<SpawnEvent<Marker>>,
    keyboard: Res<Input<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::M) {
        if let Selection::Single(GenericID::Track(track_id)) = selection_state.selection {
            let marker = Marker::new(track_id, MarkerColor::Any);
            marker_events.send(SpawnEvent(marker));
        }
    }
}

pub fn spawn_marker(
    mut commands: Commands,
    mut marker_events: EventReader<SpawnEvent<Marker>>,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in marker_events.read() {
        let marker = event.0.clone();
        let track_id = marker.track;
        let track_entity = entity_map.tracks.get(&marker.track).unwrap().clone();
        commands.entity(track_entity.clone()).insert(marker);
        entity_map.add_marker(track_id, track_entity);
    }
}

pub fn despawn_marker(
    mut commands: Commands,
    mut marker_events: EventReader<DespawnEvent<Marker>>,
    mut entity_map: ResMut<EntityMap>,
    mut marker_map: ResMut<MarkerMap>,
) {
    for event in marker_events.read() {
        let marker = event.0.clone();
        let track_id = marker.track;
        let track_entity = entity_map.tracks.get(&marker.track).unwrap().clone();
        commands.entity(track_entity.clone()).remove::<Marker>();
        entity_map.remove_marker(track_id);
        marker_map.remove_marker(track_id);
    }
}

pub struct MarkerPlugin;

impl Plugin for MarkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEvent<Marker>>();
        app.add_event::<DespawnEvent<Marker>>();
        app.register_component_as::<dyn Selectable, Marker>();
        app.add_systems(Update, (create_marker, delete_selection::<Marker>));
        app.add_systems(
            PostUpdate,
            (
                spawn_marker
                    .run_if(on_event::<SpawnEvent<Marker>>())
                    .after(spawn_track),
                despawn_marker,
            ),
        );
    }
}
