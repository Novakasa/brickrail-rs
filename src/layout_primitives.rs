use core::fmt;
use std::f32::consts::PI;

use bevy::prelude::*;
use strum_macros::EnumIter;

use crate::utils::distance_to_segment;

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect)]
pub struct WagonID {
    train: TrainID,
    index: usize,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, Serialize, Deserialize,
)]
pub struct TrainID {
    id: usize,
}

impl TrainID {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Debug,
    Reflect,
    Default,
    Serialize,
    Deserialize,
)]
pub enum BlockDirection {
    #[default]
    Aligned,
    Opposite,
}

impl BlockDirection {
    pub fn opposite(&self) -> BlockDirection {
        match self {
            BlockDirection::Aligned => BlockDirection::Opposite,
            BlockDirection::Opposite => BlockDirection::Aligned,
        }
    }

    fn get_name(&self) -> &'static str {
        match self {
            BlockDirection::Aligned => "->",
            BlockDirection::Opposite => "<-",
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize)]
pub struct BlockID {
    track1: DirectedTrackID,
    track2: DirectedTrackID,
}

impl BlockID {
    pub fn new(track1: DirectedTrackID, track2: DirectedTrackID) -> Self {
        if track2.cell() < track1.cell() {
            Self::new(track2, track1)
        } else {
            Self { track1, track2 }
        }
    }

    pub fn to_logical(&self, dir: BlockDirection, facing: Facing) -> LogicalBlockID {
        LogicalBlockID {
            block: self.clone(),
            direction: dir,
            facing,
        }
    }

    pub fn logical_block_ids(&self) -> [LogicalBlockID; 4] {
        use {BlockDirection::*, Facing::*};
        [
            self.to_logical(Aligned, Forward),
            self.to_logical(Aligned, Backward),
            self.to_logical(Opposite, Forward),
            self.to_logical(Opposite, Backward),
        ]
    }
    pub fn get_name(&self) -> String {
        format!("({})-({})", self.track1.get_name(), self.track2.get_name(),)
    }
}

impl fmt::Debug for BlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B[{}]", self.get_name())
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize)]
pub struct LogicalBlockID {
    pub block: BlockID,
    pub direction: BlockDirection,
    pub facing: Facing,
}

impl LogicalBlockID {
    pub fn default_in_marker_track(&self) -> LogicalTrackID {
        use {BlockDirection::*, Facing::*};
        match (self.direction, self.facing) {
            (Aligned, Forward) => self.block.track2.opposite().get_logical(Forward),
            (Aligned, Backward) => self.block.track1.get_logical(Backward),
            (Opposite, Forward) => self.block.track1.opposite().get_logical(Forward),
            (Opposite, Backward) => self.block.track2.get_logical(Backward),
        }
    }

    pub fn get_name(&self) -> String {
        let (first, second) = match self.direction {
            BlockDirection::Aligned => (self.block.track1, self.block.track2.opposite()),
            BlockDirection::Opposite => (self.block.track2.opposite(), self.block.track1),
        };
        format!(
            "({}){}({})",
            first.get_name(),
            self.facing.get_name(),
            second.get_name(),
        )
    }
}

impl fmt::Debug for LogicalBlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LB[{}]", self.get_name())
    }
}

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

    pub fn get_delta_vec(&self, other: &Self) -> Vec2 {
        Vec2::new((other.x - self.x) as f32, (other.y - self.y) as f32)
    }

    pub fn from_vec2(pos: Vec2) -> Self {
        Self {
            x: (pos.x + 0.5).floor() as i32,
            y: (pos.y + 0.5).floor() as i32,
            l: 0,
        }
    }

    pub fn cardinal_to(&self, other: &Self) -> Option<Cardinal> {
        Some(Cardinal::from_deltas(other.x - self.x, other.y - self.y)?)
    }

    pub fn cardinal_to_slot(&self, slot: &Slot) -> Option<Cardinal> {
        if self == &slot.cell {
            return Some(slot.interface.to_cardinal());
        }
        if self == &slot.get_other_cell() {
            return Some(slot.interface.to_cardinal().opposite());
        }
        None
    }

    pub fn get_slot(&self, cardinal: Cardinal) -> Slot {
        if let Some(interface) = CellInterface::from_cardinal(cardinal) {
            return Slot {
                cell: *self,
                interface,
            };
        }
        return Slot {
            cell: self.get_neighbor(cardinal),
            interface: CellInterface::from_cardinal(cardinal.opposite()).unwrap(),
        };
    }

    pub fn get_neighbor(&self, cardinal: Cardinal) -> Self {
        Self {
            x: self.x + cardinal.dx(),
            y: self.y + cardinal.dy(),
            l: self.l,
        }
    }

    pub fn get_shared_slot(&self, other: &Self) -> Option<Slot> {
        if let Some(cardinal) = self.cardinal_to(other) {
            return Some(self.get_slot(cardinal));
        }
        None
    }

    pub fn get_vec2(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CellInterface {
    N,
    E,
}

impl CellInterface {
    fn to_cardinal(&self) -> Cardinal {
        match self {
            CellInterface::N => Cardinal::N,
            CellInterface::E => Cardinal::E,
        }
    }

    fn from_cardinal(cardinal: Cardinal) -> Option<Self> {
        match cardinal {
            Cardinal::N => Some(CellInterface::N),
            Cardinal::E => Some(CellInterface::E),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slot {
    cell: CellID,
    interface: CellInterface,
}

impl Slot {
    pub fn can_connect_to(&self, other: &Self) -> bool {
        if self.cell == other.cell {
            return self.interface != other.interface;
        }
        if self.get_other_cell() == other.cell {
            return self.interface.to_cardinal() != other.interface.to_cardinal().opposite();
        }
        if self.cell == other.get_other_cell() {
            return self.interface.to_cardinal().opposite() != other.interface.to_cardinal();
        }
        return false;
    }

    pub fn get_shared_cell(&self, other: &Self) -> Option<CellID> {
        if self.get_other_cell() == other.cell {
            return Some(other.cell);
        }
        if self.cell == other.get_other_cell() {
            return Some(self.cell);
        }
        if self.cell == other.cell {
            return Some(self.cell);
        }
        if self.get_other_cell() == other.get_other_cell() {
            return Some(self.get_other_cell());
        }
        None
    }

    pub fn get_other_cell(&self) -> CellID {
        self.cell.get_neighbor(self.interface.to_cardinal())
    }

    pub fn get_vec2(&self) -> Vec2 {
        self.cell.get_vec2() + 0.5 * self.interface.to_cardinal().get_vec2()
    }
}

#[derive(Clone, Copy, PartialEq, EnumIter, Debug)]
pub enum Cardinal {
    N,
    S,
    E,
    W,
}

impl Cardinal {
    pub fn from_deltas(dx: i32, dy: i32) -> Option<Self> {
        match (dx, dy) {
            (0, 1) => Some(Cardinal::N),
            (0, -1) => Some(Cardinal::S),
            (1, 0) => Some(Cardinal::E),
            (-1, 0) => Some(Cardinal::W),
            _ => None,
        }
    }

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

    pub fn get_vec2(&self) -> Vec2 {
        Vec2::new(self.dx() as f32, self.dy() as f32)
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
    pub fn from_cardinals(slot1: Cardinal, slot2: Cardinal) -> Option<Self> {
        if slot1 == slot2 {
            return None;
        }
        Some(match (&slot1, &slot2) {
            (Cardinal::N, Cardinal::S) => Orientation::NS,
            (Cardinal::N, Cardinal::E) => Orientation::NE,
            (Cardinal::N, Cardinal::W) => Orientation::NW,
            (Cardinal::S, Cardinal::E) => Orientation::SE,
            (Cardinal::S, Cardinal::W) => Orientation::SW,
            (Cardinal::E, Cardinal::W) => Orientation::EW,
            _ => Self::from_cardinals(slot2, slot1)?,
        })
    }

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

    pub fn get_cardinal(&self, dir: TrackDirection) -> Cardinal {
        match dir {
            TrackDirection::First => self.get_cardinals().0,
            TrackDirection::Last => self.get_cardinals().1,
        }
    }

    pub fn get_direction_to(&self, cardinal: Cardinal) -> Option<TrackDirection> {
        let (card1, card2) = self.get_cardinals();
        if cardinal == card1 {
            return Some(TrackDirection::First);
        }
        if cardinal == card2 {
            return Some(TrackDirection::Last);
        }
        return None;
    }

    pub fn turn_index(&self) -> i32 {
        match self {
            Self::EW => 4,
            Self::NE => 1,
            Self::NS => 2,
            Self::NW => 3,
            Self::SE => 7,
            Self::SW => 5,
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

    pub fn get_reversed_name(&self) -> &'static str {
        match self {
            Self::EW => "WE",
            Self::NE => "EN",
            Self::NS => "SN",
            Self::NW => "WN",
            Self::SE => "ES",
            Self::SW => "WS",
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum ConnectionDirection {
    Aligned,
    Opposite,
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct TrackConnectionID {
    // DirectedTrackIDs point at each other to avoid bias
    // They are sorted according to track_a < track_b
    track_a: DirectedTrackID,
    track_b: DirectedTrackID,
}

impl TrackConnectionID {
    pub fn new(track_a: DirectedTrackID, track_b: DirectedTrackID) -> Self {
        if track_a < track_b {
            Self { track_a, track_b }
        } else {
            Self { track_b, track_a }
        }
    }

    pub fn track_a(&self) -> DirectedTrackID {
        self.track_a
    }

    pub fn track_b(&self) -> DirectedTrackID {
        self.track_b
    }

    pub fn is_continuous(&self) -> bool {
        self.track_a.to_slot() == self.track_b.to_slot() && !self.flips_facing()
    }

    pub fn flips_facing(&self) -> bool {
        self.track_a == self.track_b
    }

    pub fn to_directed(&self, dir: ConnectionDirection) -> DirectedTrackConnectionID {
        match dir {
            ConnectionDirection::Aligned => DirectedTrackConnectionID {
                from_track: self.track_a,
                to_track: self.track_b.opposite(),
            },
            ConnectionDirection::Opposite => DirectedTrackConnectionID {
                from_track: self.track_b,
                to_track: self.track_a.opposite(),
            },
        }
    }

    pub fn directed_connections(&self) -> [DirectedTrackConnectionID; 2] {
        [
            self.to_directed(ConnectionDirection::Aligned),
            self.to_directed(ConnectionDirection::Opposite),
        ]
    }

    pub fn logical_connections(&self) -> [LogicalTrackConnectionID; 4] {
        [
            self.to_directed(ConnectionDirection::Aligned)
                .to_logical(Facing::Forward),
            self.to_directed(ConnectionDirection::Aligned)
                .to_logical(Facing::Backward),
            self.to_directed(ConnectionDirection::Opposite)
                .to_logical(Facing::Forward),
            self.to_directed(ConnectionDirection::Opposite)
                .to_logical(Facing::Backward),
        ]
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct DirectedTrackConnectionID {
    pub from_track: DirectedTrackID,
    pub to_track: DirectedTrackID,
}

impl DirectedTrackConnectionID {
    pub fn new(from_track: DirectedTrackID, to_track: DirectedTrackID) -> Self {
        Self {
            from_track,
            to_track,
        }
    }
    pub fn is_continuous(&self) -> bool {
        self.from_track.to_slot() == self.to_track.from_slot() && !self.flips_facing()
    }

    pub fn flips_facing(&self) -> bool {
        self.from_track == self.to_track.opposite()
    }

    pub fn draw_with_gizmos(&self, gizmos: &mut Gizmos, scale: f32, color: Color) {
        let start = self.from_track.get_center_vec2() + self.from_track.get_delta_vec() * 0.2;
        let end = self.to_track.get_center_vec2() - self.to_track.get_delta_vec() * 0.2;
        gizmos.line_2d(start * scale, end * scale, color);
    }

    pub fn curve_index(&self) -> i32 {
        ((self.to_track.dir_index() - self.from_track.dir_index() + 12) % 8) - 4
    }

    fn curve_radius(&self) -> f32 {
        match self.curve_index().abs() {
            0 => 0.0,
            1 => 0.5 + 0.25 * 2.0_f32.sqrt(),
            2 => 0.25 * 2.0_f32.sqrt(),
            i => panic!("invalid curve_index! {:?}", i),
        }
    }

    fn curve_length(&self) -> f32 {
        self.curve_radius() * self.curve_index().abs() as f32
    }

    fn curve_center(&self) -> Vec2 {
        self.from_track.straight_end()
            - self.from_track.normal() * self.curve_radius() * self.curve_index().signum() as f32
    }

    fn curve_delta_angle(&self) -> f32 {
        -self.curve_index() as f32 * 0.25 * PI
    }

    fn curve_start_angle(&self) -> f32 {
        let normal = self.from_track.normal() * self.curve_index() as f32;
        normal.y.atan2(normal.x)
    }

    fn interpolate_curve(&self, dist: f32) -> Vec2 {
        let angle =
            self.curve_start_angle() + self.curve_delta_angle() * dist / self.curve_length();
        self.curve_center() + Vec2::from_angle(angle) * self.curve_radius()
    }

    fn straight_length(&self) -> f32 {
        if self.curve_index() != 0 {
            return 0.0;
        }
        (self.from_track.straight_end() - self.to_track.opposite().straight_end()).length()
    }

    pub fn connection_length(&self) -> f32 {
        self.from_track.straight_length()
            + self.curve_length()
            + self.to_track.straight_length()
            + self.straight_length()
    }

    pub fn interpolate_pos(&self, dist: f32) -> Vec2 {
        if dist < self.from_track.straight_length() {
            return self.from_track.interpolate_pos(dist);
        }

        if dist - self.from_track.straight_length() < self.curve_length() {
            return self.interpolate_curve(dist - self.from_track.straight_length());
        }

        return self
            .to_track
            .interpolate_pos(dist - self.connection_length());
    }

    pub fn to_logical(&self, from_facing: Facing) -> LogicalTrackConnectionID {
        let to_facing = if self.flips_facing() {
            from_facing.opposite()
        } else {
            from_facing
        };
        LogicalTrackConnectionID {
            from_track: self.from_track.get_logical(from_facing),
            to_track: self.to_track.get_logical(to_facing),
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct LogicalTrackConnectionID {
    pub from_track: LogicalTrackID,
    pub to_track: LogicalTrackID,
}

impl LogicalTrackConnectionID {
    pub fn new(from_track: LogicalTrackID, to_track: LogicalTrackID) -> Self {
        Self {
            from_track,
            to_track,
        }
    }

    pub fn to_directed(&self) -> DirectedTrackConnectionID {
        DirectedTrackConnectionID {
            from_track: self.from_track.dirtrack,
            to_track: self.to_track.dirtrack,
        }
    }
}

#[derive(Clone, Copy)]
enum Turn {
    Left,
    Center,
    Right,
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Reflect, Serialize, Deserialize,
)]
pub enum Facing {
    Forward,
    Backward,
}

impl Facing {
    fn opposite(&self) -> Facing {
        match self {
            Facing::Forward => Facing::Backward,
            Facing::Backward => Facing::Forward,
        }
    }

    fn get_name(&self) -> &'static str {
        match self {
            Facing::Forward => ">",
            Facing::Backward => "<",
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize, Deserialize)]
pub struct LogicalTrackID {
    pub dirtrack: DirectedTrackID,
    pub facing: Facing,
}

impl LogicalTrackID {
    pub fn cell(&self) -> CellID {
        self.dirtrack.cell()
    }

    pub fn reversed(&self) -> LogicalTrackID {
        LogicalTrackID {
            dirtrack: self.dirtrack.opposite(),
            facing: self.facing.opposite(),
        }
    }

    pub fn track(&self) -> TrackID {
        self.dirtrack.track
    }

    pub fn get_name(&self) -> String {
        format!("{}{}", self.dirtrack.get_name(), self.facing.get_name())
    }
}

impl fmt::Debug for LogicalTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "L({})", self.get_name())
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Serialize, Deserialize)]
pub struct DirectedTrackID {
    pub track: TrackID,
    pub direction: TrackDirection,
}

impl DirectedTrackID {
    pub fn opposite(&self) -> Self {
        Self {
            track: self.track,
            direction: self.direction.opposite(),
        }
    }

    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.track.distance_to(pos)
    }

    pub fn to_slot(&self) -> Slot {
        self.track
            .cell
            .get_slot(self.track.orientation.get_cardinal(self.direction))
    }

    pub fn from_slot(&self) -> Slot {
        self.track.cell.get_slot(
            self.track
                .orientation
                .get_cardinal(self.direction.opposite()),
        )
    }
    pub fn cell(&self) -> CellID {
        self.track.cell
    }

    pub fn get_center_vec2(&self) -> Vec2 {
        (self.to_slot().get_vec2() + self.from_slot().get_vec2()) * 0.5
    }

    pub fn get_delta_vec(&self) -> Vec2 {
        self.to_slot().get_vec2() - self.from_slot().get_vec2()
    }

    pub fn tangent(&self) -> Vec2 {
        self.get_delta_vec().normalize()
    }

    pub fn normal(&self) -> Vec2 {
        let tangent = self.tangent();
        Vec2::new(-tangent.y, tangent.x)
    }

    pub fn interpolate_pos(&self, dist: f32) -> Vec2 {
        self.get_center_vec2() + self.tangent() * dist
    }

    pub fn draw_with_gizmos(&self, gizmos: &mut Gizmos, scale: f32, color: Color) {
        let center_pos = self.get_center_vec2();
        let end_pos = center_pos + self.get_delta_vec() * 0.2;
        // println!("{:?} {:?}", center_pos, end_pos);
        gizmos.line_2d(center_pos * scale, end_pos * scale, color);
    }

    pub fn get_logical(&self, facing: Facing) -> LogicalTrackID {
        LogicalTrackID {
            dirtrack: *self,
            facing,
        }
    }

    pub fn logical_tracks(&self) -> [LogicalTrackID; 2] {
        [
            self.get_logical(Facing::Forward),
            self.get_logical(Facing::Backward),
        ]
    }

    pub fn dir_index(&self) -> i32 {
        match self.direction {
            TrackDirection::First => self.track.orientation.turn_index(),
            TrackDirection::Last => (self.track.orientation.turn_index() + 4) % 8,
        }
    }

    fn straight_length(&self) -> f32 {
        if self.dir_index() % 2 == 0 {
            0.5 - 0.25 * 2.0_f32.sqrt()
        } else {
            0.0
        }
    }

    fn straight_end(&self) -> Vec2 {
        self.interpolate_pos(self.straight_length())
    }

    fn get_name(&self) -> String {
        let dirstr = match self.direction {
            TrackDirection::First => self.track.orientation.get_reversed_name(),
            TrackDirection::Last => self.track.orientation.get_name(),
        };

        format!(
            "{},{},{}|{}",
            self.track.cell.x, self.track.cell.y, self.track.cell.l, dirstr
        )
    }
}

impl fmt::Debug for DirectedTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "D({})", self.get_name())
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, Serialize, Deserialize)]
pub struct TrackID {
    cell: CellID,
    orientation: Orientation,
}

impl TrackID {
    pub fn new(cell: CellID, orientation: Orientation) -> Self {
        Self { cell, orientation }
    }

    pub fn from_slots(slot1: Slot, slot2: Slot) -> Option<Self> {
        let cell = slot1.get_shared_cell(&slot2)?;
        //println!("{:?}", cell);
        let card1 = cell.cardinal_to_slot(&slot1)?;
        //println!("{:?}", card1);
        let card2 = cell.cardinal_to_slot(&slot2)?;
        //println!("{:?}", card2);
        let orientation = Orientation::from_cardinals(card1, card2)?;
        Some(Self { cell, orientation })
    }

    pub fn from_cells(cell1: CellID, cell2: CellID, cell3: CellID) -> Option<Self> {
        let slot0 = cell1.get_shared_slot(&cell2)?;
        let slot1 = cell2.get_shared_slot(&cell3)?;
        Self::from_slots(slot0, slot1)
    }

    pub fn get_directed(&self, dir: TrackDirection) -> DirectedTrackID {
        DirectedTrackID {
            track: *self,
            direction: dir,
        }
    }

    pub fn get_directed_to_cardinal(&self, cardinal: Cardinal) -> Option<DirectedTrackID> {
        Some(DirectedTrackID {
            track: *self,
            direction: self.orientation.get_direction_to(cardinal)?,
        })
    }

    pub fn get_connection_to(&self, other: TrackID) -> Option<TrackConnectionID> {
        let cardinal = self.cell.cardinal_to(&other.cell)?;
        let track1 = self.get_directed_to_cardinal(cardinal)?;
        let track2 = other.get_directed_to_cardinal(cardinal.opposite())?;
        Some(TrackConnectionID {
            track_a: track1,
            track_b: track2,
        })
    }

    pub fn dirtracks(&self) -> [DirectedTrackID; 2] {
        [
            self.get_directed(TrackDirection::First),
            self.get_directed(TrackDirection::Last),
        ]
    }

    pub fn logical_tracks(&self) -> [LogicalTrackID; 4] {
        use {Facing::*, TrackDirection::*};
        [
            self.get_directed(First).get_logical(Forward),
            self.get_directed(First).get_logical(Backward),
            self.get_directed(Last).get_logical(Forward),
            self.get_directed(Last).get_logical(Backward),
        ]
    }

    pub fn distance_to(&self, normalized_pos: Vec2) -> f32 {
        let directed = self.get_directed(TrackDirection::First);
        distance_to_segment(
            normalized_pos,
            directed.from_slot().get_vec2(),
            directed.to_slot().get_vec2(),
        )
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
}

impl fmt::Debug for TrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "T({})", self.get_name())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_slot_connectivity() {
        let slot1 = Slot {
            cell: CellID { x: 0, y: 0, l: 0 },
            interface: CellInterface::N,
        };

        for cardinal in Cardinal::iter() {
            let slot2 = slot1.cell.get_slot(cardinal);
            if cardinal == Cardinal::N {
                assert!(!slot1.can_connect_to(&slot2));
                assert!(!slot2.can_connect_to(&slot1));
            } else {
                assert!(slot1.can_connect_to(&slot2));
                assert!(slot2.can_connect_to(&slot1));
            }
        }

        let slot1 = Slot {
            cell: CellID { x: 0, y: 0, l: 0 },
            interface: CellInterface::N,
        };

        let cell2 = CellID { x: 0, y: 2, l: 0 };

        for cardinal in Cardinal::iter() {
            let slot2 = cell2.get_slot(cardinal);
            if cardinal == Cardinal::S {
                assert!(slot1.can_connect_to(&slot2));
                assert!(slot2.can_connect_to(&slot1));
            } else {
                assert!(!slot1.can_connect_to(&slot2));
                assert!(!slot2.can_connect_to(&slot1));
            }
        }
    }

    #[test]
    fn test_shared_slot() {
        let cell1 = CellID::new(0, 0, 0);

        for cardinal in Cardinal::iter() {
            let cell2 = cell1.get_neighbor(cardinal);
            assert_eq!(
                cell1.get_shared_slot(&cell2),
                Some(cell1.get_slot(cardinal))
            );

            let cell3 = cell2.get_neighbor(cardinal);
            assert_eq!(cell1.get_shared_slot(&cell3), None,);
        }
    }

    #[test]
    fn test_get_track() {
        let cell1 = CellID::new(0, 0, 0);
        let cell2 = CellID::new(1, 0, 0);
        let cell3 = CellID::new(2, 0, 0);

        let slot1 = cell1.get_slot(Cardinal::E);
        let slot2 = cell2.get_slot(Cardinal::E);

        assert_eq!(slot1.get_shared_cell(&slot2), Some(cell2));

        assert_eq!(cell1.cardinal_to(&cell2), Some(Cardinal::E));
        assert_eq!(cell2.cardinal_to(&cell1), Some(Cardinal::W));

        assert_eq!(cell2.cardinal_to(&cell3), Some(Cardinal::E));
        assert_eq!(cell3.cardinal_to(&cell2), Some(Cardinal::W));

        let track = TrackID::from_slots(slot1, slot2);
        assert_eq!(track, Some(TrackID::new(cell2, Orientation::EW)));

        let track = TrackID::from_cells(cell1, cell2, cell3);
        assert_eq!(track, Some(TrackID::new(cell2, Orientation::EW)));

        let track = TrackID::from_cells(cell1, cell2, cell1);
        assert_eq!(track, None);
    }

    #[test]
    fn test_get_diagonal_track() {
        let cell1 = CellID::new(0, 0, 0);
        let cell2 = CellID::new(1, 0, 0);
        let cell3 = CellID::new(1, -1, 0);

        let slot1 = cell1.get_slot(cell1.cardinal_to(&cell2).unwrap());
        let slot2 = cell2.get_slot(cell2.cardinal_to(&cell3).unwrap());

        assert_eq!(slot1.get_shared_cell(&slot2), Some(cell2));

        let track = TrackID::from_cells(cell1, cell2, cell3);
        assert_eq!(track, Some(TrackID::new(cell2, Orientation::SW)));
    }

    #[test]
    fn test_infinity() {
        assert!(f32::INFINITY > 1000.0);
        assert!(f32::INFINITY + 1000.0 == f32::INFINITY);
        assert!(f32::INFINITY.is_infinite());
    }
}
