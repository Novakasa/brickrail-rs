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

    pub fn get_neighbor(&self, cardinal: Cardinal) -> Self {
        Self {
            x: self.x + cardinal.dx(),
            y: self.y + cardinal.dy(),
            l: self.l,
        }
    }

    pub fn cardinal_to(&self, other: &Self) -> Option<Cardinal> {
        Cardinal::from_deltas(other.x - self.x, other.y - self.y)
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

    pub fn dx(&self) -> i32 {
        match self {
            Cardinal::E => 1,
            Cardinal::W => -1,
            _ => 0,
        }
    }

    pub fn dy(&self) -> i32 {
        match self {
            Cardinal::N => 1,
            Cardinal::S => -1,
            _ => 0,
        }
    }

    pub fn from_deltas(dx: i32, dy: i32) -> Option<Self> {
        match (dx, dy) {
            (0, 1) => Some(Cardinal::N),
            (0, -1) => Some(Cardinal::S),
            (1, 0) => Some(Cardinal::E),
            (-1, 0) => Some(Cardinal::W),
            _ => None,
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

    /// Returns which direction (First/Last) faces the given cardinal, if any.
    pub fn get_direction_to(&self, cardinal: Cardinal) -> Option<TrackDirection> {
        let (card1, card2) = self.get_cardinals();
        if cardinal == card1 {
            return Some(TrackDirection::First);
        }
        if cardinal == card2 {
            return Some(TrackDirection::Last);
        }
        None
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

impl TrackID {
    /// Returns this track directed toward the given cardinal, if the orientation faces it.
    pub fn get_directed_to(&self, cardinal: Cardinal) -> Option<DirectedTrackID> {
        Some(DirectedTrackID {
            track: *self,
            direction: self.orientation.get_direction_to(cardinal)?,
        })
    }

    /// Returns the connection between this track and another, if they are in adjacent cells
    /// and both have orientations facing the shared edge.
    /// Both directed tracks in the result point toward each other across the shared edge.
    pub fn get_connection_to(&self, other: TrackID) -> Option<TrackConnectionID> {
        let cardinal = self.cell.cardinal_to(&other.cell)?;
        let track_a = self.get_directed_to(cardinal)?;
        let track_b = other.get_directed_to(cardinal.opposite())?;
        Some(TrackConnectionID::new(track_a, track_b))
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

/// A track with a travel direction. The direction indicates which cardinal end
/// the travel is heading toward (First = first cardinal of orientation, Last = second).
#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect, Serialize, Deserialize)]
pub struct DirectedTrackID {
    pub track: TrackID,
    pub direction: TrackDirection,
}

impl DirectedTrackID {
    pub fn new(track: TrackID, direction: TrackDirection) -> Self {
        Self { track, direction }
    }

    /// The cardinal direction this directed track is heading toward.
    pub fn to_cardinal(&self) -> Cardinal {
        let (card1, card2) = self.track.orientation.get_cardinals();
        match self.direction {
            TrackDirection::First => card1,
            TrackDirection::Last => card2,
        }
    }

    /// The cardinal direction this directed track is coming from.
    pub fn from_cardinal(&self) -> Cardinal {
        self.to_cardinal().opposite()
    }

    pub fn opposite(&self) -> Self {
        Self {
            track: self.track,
            direction: self.direction.opposite(),
        }
    }
}

/// Identifies a physical connection between two tracks at a shared cell edge.
/// Both directed tracks point toward each other across the shared edge.
/// Normalized: track_a < track_b to avoid duplicate connections.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize)]
pub struct TrackConnectionID {
    pub track_a: DirectedTrackID,
    pub track_b: DirectedTrackID,
}

impl TrackConnectionID {
    /// Creates a normalized connection ID (track_a <= track_b).
    pub fn new(a: DirectedTrackID, b: DirectedTrackID) -> Self {
        if a.track <= b.track {
            Self { track_a: a, track_b: b }
        } else {
            Self { track_a: b, track_b: a }
        }
    }

    /// Returns the other directed track in this connection.
    pub fn other(&self, track: TrackID) -> Option<DirectedTrackID> {
        if self.track_a.track == track {
            Some(self.track_b)
        } else if self.track_b.track == track {
            Some(self.track_a)
        } else {
            None
        }
    }

    /// A connection is continuous (non-portal) if the two tracks are in adjacent cells
    /// and both face the shared edge. Portal connections link tracks that aren't
    /// spatially adjacent.
    pub fn is_continuous(&self) -> bool {
        let Some(cardinal) = self.track_a.track.cell.cardinal_to(&self.track_b.track.cell) else {
            return false;
        };
        self.track_a.to_cardinal() == cardinal && self.track_b.to_cardinal() == cardinal.opposite()
    }

    pub fn is_portal(&self) -> bool {
        !self.is_continuous()
    }
}

/// Color of a physical marker tile. Used by the train's color sensor for
/// detection and validation. `None` acts as a wildcard (any color matches).
#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Reflect, Serialize,
    Deserialize,
)]
pub enum MarkerColor {
    #[default]
    None,
    Red,
    Yellow,
    Green,
    Blue,
    Cyan,
    White,
    Black,
}

/// Identifies a block by the endpoint tracks of its section. Normalized: track_a <= track_b.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Reflect, Serialize, Deserialize)]
pub struct BlockID {
    pub track_a: TrackID,
    pub track_b: TrackID,
}

impl BlockID {
    /// Creates a normalized block ID (track_a <= track_b).
    pub fn new(a: TrackID, b: TrackID) -> Self {
        if a <= b {
            Self { track_a: a, track_b: b }
        } else {
            Self { track_a: b, track_b: a }
        }
    }
}

/// Discrete speed level for trains.
#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Reflect, Serialize, Deserialize,
)]
pub enum TrainSpeed {
    Slow,
    Cruise,
    Fast,
}

/// Numeric train identifier. Human-readable name is stored in `TrainData`.
#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Reflect, Serialize,
    Deserialize,
)]
pub struct TrainID(pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_id_round_trip() {
        let track = TrackID::new(CellID::new(33, 30, -53), Orientation::NE);
        assert_eq!(track, TrackID::from_name(&track.get_name()).unwrap());
    }

    #[test]
    fn adjacent_tracks_connect() {
        let t1 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
        let t2 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
        let conn = t1.get_connection_to(t2).expect("should connect");
        // t1 points east (Last for EW), t2 points west (First for EW, but actually Last..?)
        // EW cardinals are (E, W). t1 directed toward E = First direction.
        // t2 directed toward W (opposite of E) = Last direction.
        assert_eq!(conn.track_a.track, t1);
        assert_eq!(conn.track_b.track, t2);
    }

    #[test]
    fn non_adjacent_tracks_no_connection() {
        let t1 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
        let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
        assert!(t1.get_connection_to(t2).is_none());
    }

    #[test]
    fn incompatible_orientation_no_connection() {
        // NS track can't connect eastward
        let t1 = TrackID::new(CellID::new(0, 0, 0), Orientation::NS);
        let t2 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
        assert!(t1.get_connection_to(t2).is_none());
    }

    #[test]
    fn connection_id_normalized() {
        let t1 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
        let t2 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
        let conn_a = t1.get_connection_to(t2).unwrap();
        let conn_b = t2.get_connection_to(t1).unwrap();
        assert_eq!(conn_a, conn_b);
    }

    #[test]
    fn adjacent_connection_is_continuous() {
        let t1 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
        let t2 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
        let conn = t1.get_connection_to(t2).unwrap();
        assert!(conn.is_continuous());
        assert!(!conn.is_portal());
    }

    #[test]
    fn portal_connection_is_not_continuous() {
        // Manually construct a connection between non-adjacent tracks (a portal)
        let t1 = DirectedTrackID::new(
            TrackID::new(CellID::new(0, 0, 0), Orientation::EW),
            TrackDirection::First,
        );
        let t2 = DirectedTrackID::new(
            TrackID::new(CellID::new(5, 5, 0), Orientation::EW),
            TrackDirection::Last,
        );
        let conn = TrackConnectionID::new(t1, t2);
        assert!(!conn.is_continuous());
        assert!(conn.is_portal());
    }
}
