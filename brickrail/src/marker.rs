use bevy::{
    gizmos::gizmos::Gizmos,
    prelude::*,
    reflect::{Reflect, TypeRegistry},
    render::color::Color,
    utils::HashMap,
};
use bevy_egui::egui;
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
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Default, Serialize, Deserialize,
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogicalMarkerData {
    pub speed: MarkerSpeed,
}

#[derive(Debug, Component, Serialize, Deserialize, Clone)]
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

    fn inspector_ui(&mut self, context: &mut InspectorContext) {
        context.ui.label("Inspectable marker lol");
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
