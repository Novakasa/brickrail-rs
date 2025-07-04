use core::fmt;
use std::{f32::consts::PI, str::FromStr};

use bevy::{prelude::*, utils::hashbrown::HashSet};
use strum_macros::{Display, EnumIter};

use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};

use crate::utils::distance_to_segment;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize, Hash)]
pub struct ScheduleID {
    pub id: usize,
}

impl ScheduleID {
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

impl fmt::Display for ScheduleID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Schedule{}", self.id)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize, Hash)]
pub enum DestinationID {
    Random,
    Specific(usize),
}

impl fmt::Display for DestinationID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DestinationID::Random => write!(f, "Random destination"),
            DestinationID::Specific(id) => write!(f, "Destination{}", id),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize, Hash)]
pub struct LogicalDiscriminator {
    pub direction: TrackDirection,
    pub facing: Facing,
}

#[derive(
    Debug, Reflect, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy,
)]
pub enum SwitchPosition {
    Left,
    Center,
    Right,
}

impl SwitchPosition {
    pub fn opposite(&self) -> SwitchPosition {
        match self {
            SwitchPosition::Left => SwitchPosition::Right,
            SwitchPosition::Center => SwitchPosition::Center,
            SwitchPosition::Right => SwitchPosition::Left,
        }
    }
}

impl fmt::Display for SwitchPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwitchPosition::Left => write!(f, "Left"),
            SwitchPosition::Center => write!(f, "Center"),
            SwitchPosition::Right => write!(f, "Right"),
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect)]
pub struct WagonID {
    pub train: TrainID,
    pub index: usize,
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

impl fmt::Display for TrainID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Train{}", self.id)
    }
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Display,
    Reflect,
    Serialize,
    Deserialize,
)]
pub enum HubPort {
    A,
    B,
    C,
    D,
    E,
    F,
}

impl HubPort {
    pub fn to_u8(&self) -> u8 {
        match self {
            HubPort::A => 0,
            HubPort::B => 1,
            HubPort::C => 2,
            HubPort::D => 3,
            HubPort::E => 4,
            HubPort::F => 5,
        }
    }

    pub fn iter() -> impl Iterator<Item = HubPort> {
        [
            HubPort::A,
            HubPort::B,
            HubPort::C,
            HubPort::D,
            HubPort::E,
            HubPort::F,
        ]
        .iter()
        .copied()
    }
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Reflect, Serialize, Deserialize,
)]
pub enum HubType {
    Train,
    Layout,
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Reflect,
    SerializeDisplay,
    DeserializeFromStr,
)]
pub struct HubID {
    pub id: usize,
    pub kind: HubType,
}

impl HubID {
    pub fn new(id: usize, kind: HubType) -> Self {
        Self { id, kind }
    }
}

impl fmt::Display for HubID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}{}", self.kind, self.id)
    }
}

impl FromStr for HubID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("Train") {
            Ok(Self::new(s[5..].parse().unwrap(), HubType::Train))
        } else if s.starts_with("Layout") {
            Ok(Self::new(s[6..].parse().unwrap(), HubType::Layout))
        } else {
            Err(format!("invalid hub id: {}", s))
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize)]
pub enum LayoutDeviceType {
    #[serde(alias = "Switch", alias = "SwitchMotor")]
    PulseMotor,
    Signal,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize)]
pub struct LayoutDeviceID {
    pub id: usize,
    pub kind: LayoutDeviceType,
}

impl LayoutDeviceID {
    pub fn new(id: usize, kind: LayoutDeviceType) -> Self {
        Self { id, kind }
    }
}

impl fmt::Display for LayoutDeviceID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}{}", self.kind, self.id)
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

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, SerializeDisplay, DeserializeFromStr,
)]
pub struct BlockID {
    pub track1: DirectedTrackID,
    pub track2: DirectedTrackID,
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

    pub fn from_name(name: &str) -> Option<Self> {
        // println!("parsing block name: {}", name);
        let (track1, track2) = name.split_at(name.find(")-(")? + 1);
        // println!("track1: {}, track2: {}", track1, track2);
        let track1 = &track1[1..track1.len() - 1];
        let track2 = &track2[2..track2.len() - 1];
        // println!("track1: {}, track2: {}", track1, track2);
        Some(Self {
            track1: DirectedTrackID::from_name(track1)?,
            track2: DirectedTrackID::from_name(track2)?,
        })
    }
}

impl fmt::Display for BlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B[{}]", self.get_name())
    }
}

impl fmt::Debug for BlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "B[{}]", self.get_name())
    }
}

impl FromStr for BlockID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip B[ and ]:
        let s = &s[2..s.len() - 1];
        // println!("parsing block id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid block id: {}", s))
    }
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, SerializeDisplay, DeserializeFromStr,
)]
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
            BlockDirection::Opposite => (self.block.track2, self.block.track1.opposite()),
        };
        format!(
            "({}){}({})",
            first.get_name(),
            self.facing.get_name(),
            second.get_name(),
        )
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let parts = name.split(&['(', ')']).collect::<Vec<&str>>();
        let first = parts.get(1)?;
        let facing = parts.get(2)?;
        let second = parts.get(3)?;

        let first_track = DirectedTrackID::from_name(first)?;
        let second_track = DirectedTrackID::from_name(second)?;

        let block_id = BlockID::new(first_track, second_track.opposite());
        let direction = if block_id.track1 == first_track {
            BlockDirection::Aligned
        } else {
            BlockDirection::Opposite
        };
        let block_id = match direction {
            BlockDirection::Aligned => block_id,
            BlockDirection::Opposite => BlockID::new(second_track.opposite(), first_track),
        };
        let facing = Facing::from_name(facing)?;
        Some(Self {
            block: block_id,
            direction,
            facing,
        })
    }
}

impl fmt::Debug for LogicalBlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LB[{}]", self.get_name())
    }
}

impl fmt::Display for LogicalBlockID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LB[{}]", self.get_name())
    }
}

impl FromStr for LogicalBlockID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip LB[ and ]:
        let s = &s[3..s.len() - 1];
        // println!("parsing logical block id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid logical block id: {}", s))
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

    pub fn get_opposite_slot(&self, slot: &Slot) -> Option<Slot> {
        let cardinal = self.cardinal_to_slot(slot)?;
        Some(self.get_slot(cardinal.opposite()))
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

    pub fn from_reversed_name(name: &str) -> Option<Self> {
        match name {
            "WE" => Some(Self::EW),
            "EN" => Some(Self::NE),
            "SN" => Some(Self::NS),
            "WN" => Some(Self::NW),
            "ES" => Some(Self::SE),
            "WS" => Some(Self::SW),
            _ => None,
        }
    }

    pub fn get_unicode_arrow(&self) -> &'static str {
        match self {
            Self::EW => "↔",
            Self::NE => "⤡",
            Self::NS => "↕",
            Self::NW => "⤢",
            Self::SE => "⤢",
            Self::SW => "⤡",
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct DirectedConnectionShape {
    pub orientation: Orientation,
    pub direction: TrackDirection,
    pub turn: SwitchPosition,
    pub is_portal: bool,
}

impl DirectedConnectionShape {
    pub fn to_connection(&self, cell: CellID) -> DirectedTrackConnectionID {
        let directed_track = DirectedTrackID {
            track: TrackID {
                cell,
                orientation: self.orientation,
            },
            direction: self.direction,
        };
        DirectedTrackConnectionID {
            from_track: directed_track,
            to_track: directed_track.get_next_track(&self.turn),
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub enum ConnectionDirection {
    Aligned,
    Opposite,
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, SerializeDisplay, DeserializeFromStr,
)]
pub struct TrackConnectionID {
    // DirectedTrackIDs point at each other to avoid bias
    // They are sorted according to track_a < track_b
    pub track_a: DirectedTrackID,
    pub track_b: DirectedTrackID,
}

impl TrackConnectionID {
    pub fn new(track_a: DirectedTrackID, track_b: DirectedTrackID) -> Self {
        if track_a < track_b {
            Self { track_a, track_b }
        } else {
            Self { track_b, track_a }
        }
    }

    pub fn tracks(&self) -> [DirectedTrackID; 2] {
        [self.track_a, self.track_b]
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

    pub fn get_name(&self) -> String {
        format!("{}><{}", self.track_a.get_name(), self.track_b.get_name())
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let split = name.split("><").collect::<Vec<&str>>();
        let track1 = DirectedTrackID::from_name(split.get(0)?)?;
        let track2 = DirectedTrackID::from_name(split.get(1)?)?;
        Some(Self::new(track1, track2))
    }
}

impl fmt::Display for TrackConnectionID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "C({})", self.get_name())
    }
}

impl fmt::Debug for TrackConnectionID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "C({})", self.get_name())
    }
}

impl FromStr for TrackConnectionID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip C( and ):
        let s = &s[2..s.len() - 1];
        // println!("parsing track connection id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid track connection id: {}", s))
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

    pub fn shape_id(&self) -> DirectedConnectionShape {
        DirectedConnectionShape {
            orientation: self.from_track.track.orientation,
            direction: self.from_track.direction,
            turn: self.get_switch_position(),
            is_portal: !self.is_continuous(),
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
        if !self.is_continuous() {
            return 0;
        }
        ((self.to_track.dir_index() - self.from_track.dir_index() + 12) % 8) - 4
    }

    fn curve_radius(&self) -> f32 {
        match self.curve_index().abs() {
            0 => 0.0,
            1 => 0.5 + 0.25 * 2.0_f32.sqrt(),
            2 => 0.25 * 2.0_f32.sqrt(),
            _ => 0.0,
        }
    }

    fn curve_length(&self) -> f32 {
        self.curve_radius() * self.curve_delta_angle().abs()
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

    pub fn straight_length(&self) -> f32 {
        if self.curve_index() != 0 {
            return 0.0;
        }
        (self.from_track.straight_end() - self.to_track.opposite().straight_end()).length()
    }

    pub fn connection_length(&self) -> f32 {
        if !self.is_continuous() {
            return 0.8;
        }
        self.from_track.straight_length()
            + self.curve_length()
            + self.to_track.straight_length()
            + self.straight_length()
    }

    pub fn interpolate_pos(&self, dist: f32) -> Vec2 {
        if !self.is_continuous() {
            if dist < self.connection_length() * 0.5 {
                return self.from_track.interpolate_pos(dist);
            }
            return self
                .to_track
                .opposite()
                .interpolate_pos(self.connection_length() - dist);
        }
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

    pub fn to_connection(&self) -> TrackConnectionID {
        TrackConnectionID {
            track_a: self.from_track,
            track_b: self.to_track.opposite(),
        }
    }

    pub fn get_switch_position(&self) -> SwitchPosition {
        if !self.is_continuous() {
            self.from_track.get_switch_position().opposite()
        } else {
            self.to_track.get_switch_position()
        }
    }

    pub fn opposite(&self) -> Self {
        Self {
            from_track: self.to_track.opposite(),
            to_track: self.from_track.opposite(),
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

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Reflect,
    Serialize,
    Deserialize,
    Default,
)]
#[reflect(Default)]
pub enum Facing {
    #[default]
    Forward,
    Backward,
}

impl Facing {
    pub fn get_sign(&self) -> f32 {
        match self {
            Facing::Forward => 1.0,
            Facing::Backward => -1.0,
        }
    }
    pub fn opposite(&self) -> Facing {
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

    fn from_name(name: &str) -> Option<Self> {
        match name {
            ">" => Some(Facing::Forward),
            "<" => Some(Facing::Backward),
            _ => None,
        }
    }

    pub fn as_train_flag(&self) -> u8 {
        match self {
            Facing::Forward => 0,
            Facing::Backward => 1,
        }
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SerializeDisplay, DeserializeFromStr,
)]
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

    pub fn discriminator(&self) -> LogicalDiscriminator {
        LogicalDiscriminator {
            direction: self.dirtrack.direction,
            facing: self.facing,
        }
    }

    pub fn get_name(&self) -> String {
        format!("{}{}", self.dirtrack.get_name(), self.facing.get_name())
    }

    pub fn from_name(name: &str) -> Option<Self> {
        // split into last char and rest str:
        let (dirtrack, facing) = name.split_at(name.len() - 1);
        let facing = Facing::from_name(facing)?;
        let dirtrack = DirectedTrackID::from_name(dirtrack)?;
        Some(Self { dirtrack, facing })
    }

    pub fn is_default(&self) -> bool {
        self.facing == Facing::Forward && self.dirtrack.direction == TrackDirection::First
    }

    pub fn get_dirstring(&self) -> String {
        let dirname = match self.dirtrack.dir_index() {
            0 => "E",
            1 => "SE",
            2 => "S",
            3 => "SW",
            4 => "W",
            5 => "NW",
            6 => "N",
            7 => "NE",
            _ => panic!("invalid dir index"),
        };
        format!("{:?}s towards {}", self.facing, dirname)
    }
}

impl fmt::Debug for LogicalTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L({}|{})",
            self.get_name(),
            self.dirtrack.get_unicode_arrow()
        )
    }
}

impl fmt::Display for LogicalTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L({}|{})",
            self.get_name(),
            self.dirtrack.get_unicode_arrow()
        )
    }
}

impl FromStr for LogicalTrackID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip L( and ):
        let end_index = s.char_indices().nth_back(2).map(|(i, _)| i).unwrap();
        let s = &s[2..end_index];
        // println!("parsing logical track id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid logical track id: {}", s))
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

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, SerializeDisplay, DeserializeFromStr,
)]
pub struct DirectedTrackID {
    pub track: TrackID,
    pub direction: TrackDirection,
}

impl DirectedTrackID {
    pub fn from_slots(from_slot: Slot, to_slot: Slot) -> Option<Self> {
        let track = TrackID::from_slots(from_slot, to_slot)?;
        track.get_directed_to_slot(to_slot)
    }

    pub fn get_switch_position(&self) -> SwitchPosition {
        let opposite_from_slot = self
            .track
            .cell
            .get_opposite_slot(&self.from_slot())
            .unwrap();
        let center_track =
            DirectedTrackID::from_slots(self.from_slot(), opposite_from_slot).unwrap();

        let delta = ((self.dir_index() - center_track.dir_index() + 12) % 8) - 4;
        match delta {
            1 => SwitchPosition::Right,
            -1 => SwitchPosition::Left,
            0 => SwitchPosition::Center,
            i => panic!("invalid switch position {:?}", i),
        }
    }

    pub fn get_next_track(&self, position: &SwitchPosition) -> DirectedTrackID {
        let delta = match position {
            SwitchPosition::Right => -1.0f32,
            SwitchPosition::Left => 1.0,
            SwitchPosition::Center => 0.0,
        };
        let cell = self.track.cell.get_neighbor(self.to_cardinal());
        let straight_dir = self.track.cell.get_delta_vec(&cell);
        let orthogonal_dir = Vec2::new(-straight_dir.y, straight_dir.x);
        let final_cell = CellID::from_vec2(
            cell.get_vec2() + straight_dir * (1.0 - delta.abs()) + orthogonal_dir * delta,
        );
        let to_slot = cell.get_shared_slot(&final_cell).unwrap();
        DirectedTrackID::from_slots(self.to_slot(), to_slot).unwrap()
    }

    pub fn get_switch_connection(
        &self,
        switch_position: &SwitchPosition,
    ) -> DirectedTrackConnectionID {
        let switch_track = self.get_next_track(&switch_position);
        DirectedTrackConnectionID {
            from_track: self.clone(),
            to_track: switch_track,
        }
    }

    pub fn opposite(&self) -> Self {
        Self {
            track: self.track,
            direction: self.direction.opposite(),
        }
    }

    pub fn distance_to(&self, pos: Vec2) -> f32 {
        self.track.distance_to(pos)
    }

    pub fn to_cardinal(&self) -> Cardinal {
        self.track.orientation.get_cardinal(self.direction)
    }

    pub fn from_cardinal(&self) -> Cardinal {
        self.track
            .orientation
            .get_cardinal(self.direction.opposite())
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
            TrackDirection::Last => self.track.orientation.turn_index(),
            TrackDirection::First => (self.track.orientation.turn_index() + 4) % 8,
        }
    }

    pub fn straight_length(&self) -> f32 {
        if self.dir_index() % 2 == 0 {
            0.5 - 0.25 * 2.0_f32.sqrt()
        } else {
            0.0
        }
    }

    fn straight_end(&self) -> Vec2 {
        self.interpolate_pos(self.straight_length())
    }

    pub fn get_unicode_arrow(&self) -> &'static str {
        match self.dir_index() {
            0 => "→",
            1 => "↘",
            2 => "↓",
            3 => "↙",
            4 => "←",
            5 => "↖",
            6 => "↑",
            7 => "↗",
            _ => "X",
        }
    }

    pub fn get_name(&self) -> String {
        let dirstr = match self.direction {
            TrackDirection::First => self.track.orientation.get_reversed_name(),
            TrackDirection::Last => self.track.orientation.get_name(),
        };

        format!(
            "{},{},{}|{}",
            self.track.cell.x, self.track.cell.y, self.track.cell.l, dirstr
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
        let (dir, orientation) = if let Some(orientation) = Orientation::from_name(orientation) {
            (TrackDirection::Last, orientation)
        } else {
            (
                TrackDirection::First,
                Orientation::from_reversed_name(orientation)?,
            )
        };
        Some(Self {
            track: TrackID {
                cell: CellID { x, y, l },
                orientation,
            },
            direction: dir,
        })
    }
}

impl fmt::Debug for DirectedTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "D({}|{})", self.get_name(), self.get_unicode_arrow())
    }
}

impl fmt::Display for DirectedTrackID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "D({}|{})", self.get_name(), self.get_unicode_arrow())
    }
}

impl FromStr for DirectedTrackID {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // strip D( and ):
        let end_index = s.char_indices().nth_back(2).map(|(i, _)| i).unwrap();
        let s = &s[2..end_index];
        // println!("parsing directed track id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid directed track id: {}", s))
    }
}

#[derive(
    Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Reflect, SerializeDisplay, DeserializeFromStr,
)]
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

    pub fn slot0(&self) -> Slot {
        self.cell.get_slot(self.orientation.get_cardinals().0)
    }

    pub fn slot1(&self) -> Slot {
        self.cell.get_slot(self.orientation.get_cardinals().1)
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

    pub fn get_directed_to_slot(&self, slot: Slot) -> Option<DirectedTrackID> {
        let cardinal = self.cell.cardinal_to_slot(&slot)?;
        self.get_directed_to_cardinal(cardinal)
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

    pub fn colliding_tracks(&self) -> HashSet<TrackID> {
        let mut tracks = HashSet::new();
        let cell = self.cell;
        for cardinal in [Cardinal::N, Cardinal::S, Cardinal::E, Cardinal::W].iter() {
            let to_slot = cell.get_slot(*cardinal);
            if let Some(track) = Self::from_slots(self.slot0(), to_slot) {
                tracks.insert(track);
            }
            if let Some(track) = Self::from_slots(self.slot1(), to_slot) {
                tracks.insert(track);
            }
        }
        match self.orientation {
            Orientation::EW => {
                tracks.insert(Self::new(cell, Orientation::NS));
            }
            Orientation::NS => {
                tracks.insert(Self::new(cell, Orientation::EW));
            }
            _ => {}
        }
        tracks
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
        // strip T( and ):
        let end_index = s.char_indices().nth_back(2).map(|(i, _)| i).unwrap();
        let s = &s[2..end_index];
        // println!("parsing track id: {}", s);
        Self::from_name(s).ok_or_else(|| format!("invalid track id: {}", s))
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

    #[test]
    fn test_to_and_from_name() {
        let track = TrackID::new(CellID::new(33, 30, -53), Orientation::NE);
        assert_eq!(track, TrackID::from_name(&track.get_name()).unwrap());

        let dirtrack = DirectedTrackID {
            track,
            direction: TrackDirection::Last,
        };
        assert_eq!(
            dirtrack,
            DirectedTrackID::from_name(&dirtrack.get_name()).unwrap()
        );

        let logical_track = LogicalTrackID {
            dirtrack,
            facing: Facing::Backward,
        };
        assert_eq!(
            logical_track,
            LogicalTrackID::from_name(&logical_track.get_name()).unwrap()
        );

        let dirtrack2 = DirectedTrackID {
            track: TrackID::new(CellID::new(0, 0, 0), Orientation::EW),
            direction: TrackDirection::First,
        };

        let block = BlockID::new(dirtrack, dirtrack2);
        assert_eq!(block, BlockID::from_name(&block.get_name()).unwrap());

        let logical_block = block.to_logical(BlockDirection::Opposite, Facing::Backward);
        let block2 = LogicalBlockID::from_name(&logical_block.get_name()).unwrap();
        println!("{:?} {:?}", logical_block.block, block2.block);
        assert_eq!(
            logical_block,
            LogicalBlockID::from_name(&logical_block.get_name()).unwrap()
        );

        let connection = TrackConnectionID::new(
            DirectedTrackID {
                track: TrackID::new(CellID::new(0, 0, 0), Orientation::EW),
                direction: TrackDirection::First,
            },
            DirectedTrackID {
                track: TrackID::new(CellID::new(0, 0, 0), Orientation::EW),
                direction: TrackDirection::Last,
            },
        );
        assert_eq!(
            connection,
            TrackConnectionID::from_name(&connection.get_name()).unwrap()
        );
    }

    #[test]
    fn parse_primitives() {
        let track1 = LogicalTrackID::from_str("L(-2,2,0|WE>)").unwrap();
        assert_eq!(track1.cell().x, -2);
        assert_eq!(track1.cell().y, 2);
        assert_eq!(track1.cell().l, 0);
        assert_eq!(track1.facing, Facing::Forward);

        let track2 = LogicalTrackID::from_str("L(-2,2,0|WE<)").unwrap();
        assert_eq!(track2.facing, Facing::Backward);
        assert_ne!(track1.facing, track2.facing);

        let dirtrack = DirectedTrackID::from_str("D(-2,2,0|NS)").unwrap();
        let dirtrack2 = DirectedTrackID::from_str("D(-2,2,0|SN)").unwrap();
        assert_eq!(dirtrack, dirtrack2.opposite());

        let block = LogicalBlockID::from_str("LB[(-1,0,0|SN)>(-1,3,0|SN)]").unwrap();
        assert_eq!(
            block.default_in_marker_track(),
            LogicalTrackID::from_str("L(-1,3,0|SN>)").unwrap(),
        );
        assert_eq!(block, LogicalBlockID::from_str(&block.to_string()).unwrap());

        let block = LogicalBlockID::from_str("LB[(-1,3,0|SN)>(-1,0,0|SN)]").unwrap();
        println!("{:?}", block);
        println!(
            "block {:?}, direction {:?}, facing {:?}",
            block.block, block.direction, block.facing
        );
        assert_eq!(block, LogicalBlockID::from_str(&block.to_string()).unwrap());
        // assert!(false);
    }
}
