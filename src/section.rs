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
}
