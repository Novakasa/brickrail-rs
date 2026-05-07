use core::fmt;
use std::str::FromStr;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect, Serialize, Deserialize,
)]
pub struct CellID {
    pub x: i32,
    pub y: i32,
    pub l: i32,
}

impl CellID {
    pub fn new(x: i32, y: i32, l: i32) -> Self {
        Self { x, y, l }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cardinal {
    N,
    S,
    E,
    W,
}

impl Cardinal {
    pub fn opposite(&self) -> Self {
        match self {
            Cardinal::N => Cardinal::S,
            Cardinal::S => Cardinal::N,
            Cardinal::E => Cardinal::W,
            Cardinal::W => Cardinal::E,
        }
    }
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect, Serialize, Deserialize,
)]
pub enum Orientation {
    NS,
    NE,
    NW,
    SE,
    SW,
    EW,
}

impl Orientation {
    pub fn get_cardinals(&self) -> (Cardinal, Cardinal) {
        match self {
            Orientation::NS => (Cardinal::N, Cardinal::S),
            Orientation::NE => (Cardinal::N, Cardinal::E),
            Orientation::NW => (Cardinal::N, Cardinal::W),
            Orientation::SE => (Cardinal::S, Cardinal::E),
            Orientation::SW => (Cardinal::S, Cardinal::W),
            Orientation::EW => (Cardinal::E, Cardinal::W),
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            Self::EW => "EW",
            Self::NE => "NE",
            Self::NS => "NS",
            Self::NW => "NW",
            Self::SE => "SE",
            Self::SW => "SW",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "EW" => Some(Self::EW),
            "NE" => Some(Self::NE),
            "NS" => Some(Self::NS),
            "NW" => Some(Self::NW),
            "SE" => Some(Self::SE),
            "SW" => Some(Self::SW),
            _ => None,
        }
    }

    pub fn get_unicode_arrow(&self) -> &'static str {
        match self {
            Self::EW => "\u{2194}",
            Self::NE => "\u{2921}",
            Self::NS => "\u{2195}",
            Self::NW => "\u{2922}",
            Self::SE => "\u{2922}",
            Self::SW => "\u{2921}",
        }
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect, Serialize, Deserialize,
)]
pub enum TrackDirection {
    First,
    Last,
}

impl TrackDirection {
    pub fn opposite(&self) -> Self {
        match self {
            TrackDirection::First => TrackDirection::Last,
            TrackDirection::Last => TrackDirection::First,
        }
    }
}

/// Identifies a track segment by its grid cell and orientation.
#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect)]
pub struct TrackID {
    pub cell: CellID,
    pub orientation: Orientation,
}

impl TrackID {
    pub fn new(cell: CellID, orientation: Orientation) -> Self {
        Self { cell, orientation }
    }

    pub fn get_name(&self) -> String {
        format!(
            "{},{},{}|{}",
            self.cell.x,
            self.cell.y,
            self.cell.l,
            self.orientation.get_name()
        )
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let mut parts = name.split('|');
        let cell = parts.next()?;
        let orientation = parts.next()?;
        let mut cell_parts = cell.split(',');
        let x = cell_parts.next()?.parse::<i32>().ok()?;
        let y = cell_parts.next()?.parse::<i32>().ok()?;
        let l = cell_parts.next()?.parse::<i32>().ok()?;
        let orientation = Orientation::from_name(orientation)?;
        Some(Self {
            cell: CellID { x, y, l },
            orientation,
        })
    }
}

impl fmt::Debug for TrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "T({}|{})",
            self.get_name(),
            self.orientation.get_unicode_arrow()
        )
    }
}

impl fmt::Display for TrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "T({}|{})",
            self.get_name(),
            self.orientation.get_unicode_arrow()
        )
    }
}

impl FromStr for TrackID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let end_index = s.char_indices().nth_back(2).map(|(i, _)| i).unwrap();
        let s = &s[2..end_index];
        Self::from_name(s).ok_or_else(|| format!("invalid track id: {}", s))
    }
}

impl Serialize for TrackID {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TrackID {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_id_round_trip() {
        let track = TrackID::new(CellID::new(33, 30, -53), Orientation::NE);
        assert_eq!(track, TrackID::from_name(&track.get_name()).unwrap());
    }
}
