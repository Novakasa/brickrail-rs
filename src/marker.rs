use bevy::{ecs::component::Component, reflect::Reflect, utils::HashMap};

use crate::layout_primitives::*;

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect)]
pub enum MarkerKey {
    Enter,
    In,
    None,
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Default)]
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum MarkerColor {
    Any,
    Red,
    Blue,
    Yellow,
    Green,
}

#[derive(Debug, Clone, Default)]
pub struct LogicalMarkerData {
    pub speed: MarkerSpeed,
}

#[derive(Debug, Component)]
pub struct Marker {
    pub track: TrackID,
    pub color: MarkerColor,
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
}
