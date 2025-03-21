use bevy::{math::Vec2, reflect::Reflect};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{layout::Connections, layout_primitives::*};

#[derive(Debug, Clone)]
pub struct TrackSection {
    pub tracks: Vec<TrackID>,
}

#[derive(Debug, Clone)]
pub struct LogicalSection {
    pub tracks: Vec<LogicalTrackID>,
}

impl LogicalSection {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn extend_merge(&mut self, other: &LogicalSection) {
        for track in other.tracks.iter() {
            if !self.tracks.contains(track) {
                self.tracks.push(track.clone());
            }
        }
    }

    pub fn split_by_tracks_with_overlap(
        &self,
        tracks: Vec<&LogicalTrackID>,
    ) -> Vec<(LogicalSection, LogicalTrackID)> {
        let mut results = vec![];
        let mut current_section = LogicalSection::new();
        for track in self.tracks.iter() {
            current_section.tracks.push(track.clone());
            if tracks.contains(&track) {
                results.push((current_section.clone(), *track));
                current_section = LogicalSection::new();
                current_section.tracks.push(track.clone());
            }
        }
        results
    }

    pub fn connection_iter(&self) -> impl Iterator<Item = LogicalTrackConnectionID> + '_ {
        self.tracks
            .iter()
            .tuple_windows()
            .map(|(a, b)| LogicalTrackConnectionID::new(*a, *b))
    }

    pub fn directed_connection_iter(&self) -> impl Iterator<Item = DirectedTrackConnectionID> + '_ {
        self.tracks
            .iter()
            .tuple_windows()
            .map(|(a, b)| DirectedTrackConnectionID::new(a.dirtrack, b.dirtrack))
    }

    pub fn length(&self) -> f32 {
        self.directed_connection_iter()
            .map(|c| c.connection_length())
            .sum()
    }

    pub fn length_to(&self, track: &LogicalTrackID) -> Result<f32, ()> {
        println!("length_to {:?}", track);
        let mut length = 0.0;
        if track == self.tracks.first().ok_or(())? {
            return Ok(0.0);
        }
        for connection in self.directed_connection_iter() {
            length += connection.connection_length();
            if connection.to_track == track.dirtrack {
                return Ok(length);
            }
        }
        return Err(());
    }

    pub fn interpolate_pos(&self, mut pos: f32) -> Vec2 {
        if self.tracks.len() == 1 {
            return self.tracks.first().unwrap().dirtrack.interpolate_pos(pos);
        }
        let mut last_pos = pos;
        let mut last_connection = self.directed_connection_iter().next().unwrap();
        for connection in self.directed_connection_iter() {
            let length = connection.connection_length();
            if pos <= length {
                return connection.interpolate_pos(pos);
            }
            last_connection = connection;
            last_pos = pos;
            pos -= length;
        }
        return last_connection.interpolate_pos(last_pos);
    }
}

#[derive(Debug, Clone, Reflect, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectedSection {
    pub tracks: Vec<DirectedTrackID>,
}

impl DirectedSection {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    pub fn push(&mut self, track: DirectedTrackID, connections: &Connections) -> Result<(), ()> {
        if self.tracks.is_empty() {
            self.tracks.push(track);
            Ok(())
        } else {
            let last_track = self.tracks.last().unwrap();
            if connections
                .has_directed_connection(&DirectedTrackConnectionID::new(*last_track, track))
            {
                self.tracks.push(track);
                Ok(())
            } else {
                Err(())
            }
        }
    }

    pub fn push_track(&mut self, track: TrackID, connections: &Connections) -> Result<(), ()> {
        for dirtrack in track.dirtracks() {
            if self.push(dirtrack, connections).is_ok() {
                return Ok(());
            }
        }
        return Err(());
    }

    pub fn get_opposite(&self) -> Self {
        let mut opposite = DirectedSection::new();
        for track in self.tracks.iter().rev() {
            opposite.tracks.push(track.opposite());
        }
        opposite
    }

    pub fn get_logical(&self, facing: Facing) -> LogicalSection {
        let mut logical = LogicalSection::new();
        for track in self.tracks.iter() {
            logical.tracks.push(track.get_logical(facing));
        }
        logical
    }

    pub fn has_directed_connection(&self, connection: &DirectedTrackConnectionID) -> bool {
        for (track_a, track_b) in self.tracks.iter().tuple_windows() {
            if connection.from_track == *track_a && connection.to_track == *track_b {
                return true;
            }
        }
        return false;
    }

    pub fn has_track(&self, track: &TrackID) -> bool {
        for dirtrack in track.dirtracks() {
            if self.tracks.contains(&dirtrack) {
                return true;
            }
        }
        return false;
    }

    pub fn has_connection(&self, connection: &TrackConnectionID) -> bool {
        for direction in [ConnectionDirection::Aligned, ConnectionDirection::Opposite].iter() {
            if self.has_directed_connection(&connection.to_directed(*direction)) {
                return true;
            }
        }
        return false;
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn length(&self) -> f32 {
        self.connection_iter().map(|c| c.connection_length()).sum()
    }

    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.tracks
            .iter()
            .map(|c| c.distance_to(pos))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }

    pub fn closest_track_index(&self, pos: Vec2) -> usize {
        self.tracks
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.distance_to(pos).partial_cmp(&b.distance_to(pos)).unwrap())
            .unwrap()
            .0
    }

    pub fn connection_iter(&self) -> impl Iterator<Item = DirectedTrackConnectionID> + '_ {
        self.tracks
            .iter()
            .tuple_windows()
            .map(|(a, b)| DirectedTrackConnectionID::new(*a, *b))
    }

    pub fn interpolate_pos(&self, mut pos: f32) -> Vec2 {
        if self.tracks.len() == 1 {
            return self.tracks.first().unwrap().interpolate_pos(pos);
        }
        let mut last_pos = pos;
        let mut last_connection = self.connection_iter().next().unwrap();
        for connection in self.connection_iter() {
            let length = connection.connection_length();
            if pos <= length {
                return connection.interpolate_pos(pos);
            }
            last_connection = connection;
            last_pos = pos;
            pos -= length;
        }
        return last_connection.interpolate_pos(last_pos);
    }

    pub fn to_block_id(&self) -> BlockID {
        BlockID::new(
            *self.tracks.first().unwrap(),
            self.tracks.last().unwrap().opposite(),
        )
    }
}
