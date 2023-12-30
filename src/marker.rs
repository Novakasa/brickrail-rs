use bevy::{
    ecs::component::Component, gizmos::gizmos::Gizmos, reflect::Reflect, render::color::Color,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

use crate::{layout_primitives::*, track::LAYOUT_SCALE};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect)]
pub enum MarkerKey {
    Enter,
    In,
    None,
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
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Serialize, Deserialize)]
pub enum MarkerColor {
    Any,
    Red,
    Blue,
    Yellow,
    Green,
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
