use itertools::Itertools;

use crate::{layout::Layout, layout_primitives::*};

#[derive(Debug, Clone)]
pub struct TrackSection {
    pub tracks: Vec<DirectedTrackID>,
}

impl TrackSection {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    pub fn push(&mut self, track: DirectedTrackID, layout: &Layout) -> Result<(), ()> {
        if self.tracks.is_empty() {
            self.tracks.push(track);
            Ok(())
        } else {
            let last_track = self.tracks.last().unwrap();
            if layout.has_directed_connection(&DirectedTrackConnectionID::new(*last_track, track)) {
                self.tracks.push(track);
                Ok(())
            } else {
                Err(())
            }
        }
    }

    pub fn push_track(&mut self, track: TrackID, layout: &Layout) -> Result<(), ()> {
        for dirtrack in track.dirtracks() {
            if self.push(dirtrack, layout).is_ok() {
                return Ok(());
            }
        }
        return Err(());
    }

    pub fn get_opposite(&self) -> Self {
        let mut opposite = TrackSection::new();
        for track in self.tracks.iter().rev() {
            opposite.tracks.push(track.opposite());
        }
        opposite
    }

    pub fn has_directed_connection(&self, connection: &DirectedTrackConnectionID) -> bool {
        for (track_a, track_b) in self.tracks.iter().tuple_windows() {
            if connection.from_track == *track_a && connection.to_track == *track_b {
                return true;
            }
        }
        return false;
    }

    pub fn has_connection(&self, connection: &TrackConnectionID) -> bool {
        for direction in [ConnectionDirection::Forward, ConnectionDirection::Backward].iter() {
            if self.has_directed_connection(&connection.to_directed(*direction)) {
                return true;
            }
        }
        return false;
    }
}
