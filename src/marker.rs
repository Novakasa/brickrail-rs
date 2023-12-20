use bevy::utils::HashMap;

use crate::layout_primitives::*;

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum MarkerKey {
    Enter(LogicalBlockID),
    In(LogicalBlockID),
    None,
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum MarkerSpeed {
    Slow,
    Cruise,
    Fast,
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum MarkerColor {
    Any,
    Red,
    Blue,
    Yellow,
    Green,
}

struct LogicalMarkerData {
    speed: MarkerSpeed,
    key: MarkerKey,
}

pub struct Marker {
    pub track: TrackID,
    pub color: MarkerColor,
    logical_data: HashMap<LogicalTrackID, LogicalMarkerData>,
}

impl Marker {
    pub fn collapse_logical(&self, logical_track: LogicalTrackID) -> Option<RouteMarkerData> {
        if logical_track.dirtrack.track != self.track {
            return None;
        }
        let logical = self.logical_data.get(&logical_track).unwrap();
        return Some(RouteMarkerData {
            track: logical_track,
            color: self.color,
            speed: logical.speed,
            key: logical.key,
        });
    }
}

#[derive(Debug, Clone)]
pub struct RouteMarkerData {
    pub track: LogicalTrackID,
    pub color: MarkerColor,
    pub speed: MarkerSpeed,
    pub key: MarkerKey,
}
