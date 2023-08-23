use bevy::utils::HashMap;

use crate::layout_primitives::*;

pub struct Route {
    legs: Vec<RouteLeg>,
}

impl Route {
    pub fn from_tracks(tracks: Vec<LogicalTrackID>, markers: &HashMap<TrackID, Marker>) -> Self {
        let mut leg_tracks: Vec<LogicalTrackID> = vec![];
        let mut leg_markers: Vec<LogicalMarker> = vec![];
        let mut edges: Vec<RouteLeg> = vec![];
        for track in tracks {
            if let Some(marker) = markers.get(&track.dirtrack.track) {
                let logical_marker = marker
                    .logicals
                    .get(&(track.dirtrack.direction, track.facing))
                    .unwrap();

                leg_markers.push(logical_marker.clone());
                if logical_marker.key == MarkerKey::In {
                    // start new leg
                    edges.push(RouteLeg {
                        tracks: leg_tracks,
                        markers: leg_markers,
                        current_marker_index: 0,
                    });
                }
            }
            leg_tracks.push(track);
        }

        Route { legs: vec![] }
    }

    pub fn next_leg(&mut self) -> bool {
        self.legs.remove(0);
        if self.legs.len() == 0 {
            return true;
        }
        return false;
    }

    pub fn advance_sensor(&mut self) -> bool {
        return if self.legs[0].advance_sensor() {
            self.next_leg()
        } else {
            false
        };
    }
}

pub struct RouteLeg {
    tracks: Vec<LogicalTrackID>,
    markers: Vec<LogicalMarker>,
    current_marker_index: usize,
}

impl RouteLeg {
    fn advance_sensor(&mut self) -> bool {
        self.current_marker_index += 1;
        if self.current_marker_index == self.markers.len() {
            return true;
        }
        return false;
    }
}
