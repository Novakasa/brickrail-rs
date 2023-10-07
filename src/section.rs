use crate::layout_primitives::*;
use bevy::prelude::*;

pub struct TrackSection {
    tracks: Vec<DirectedTrackID>,
}

impl TrackSection {
    pub fn push(&mut self, track: DirectedTrackID) -> Result<(), ()> {
        self.tracks.push(track);
        Ok(())
    }
}
