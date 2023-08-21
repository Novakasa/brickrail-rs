use bevy::prelude::*;
use strum_macros::EnumIter;

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct CellID {
    pub x: i32,
    pub y: i32,
    pub l: i32,
}

impl CellID {
    pub fn new(x: i32, y: i32, l: i32) -> Self {
        Self { x, y, l }
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
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
            TrackDirection::Aligned => self.get_cardinals().0,
            TrackDirection::Misaligned => self.get_cardinals().1,
        }
    }

    pub fn get_direction_to(&self, cardinal: Cardinal) -> Option<TrackDirection> {
        let (card1, card2) = self.get_cardinals();
        if cardinal == card1 {
            return Some(TrackDirection::Aligned);
        }
        if cardinal == card2 {
            return Some(TrackDirection::Misaligned);
        }
        return None;
    }
}

pub enum ConnectionDirection {
    Forward,
    Backward,
}

#[derive(Debug)]
pub struct TrackConnection {
    // DirectedTrackIDs point at each other to avoid bias
    // They are sorted according to track_a < track_b
    track_a: DirectedTrackID,
    track_b: DirectedTrackID,
}

impl TrackConnection {
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
        self.track_a.to_slot() == self.track_b.to_slot()
    }

    pub fn to_directed(&self, dir: ConnectionDirection) -> DirectedTrackConnection {
        match dir {
            ConnectionDirection::Forward => DirectedTrackConnection {
                from_track: self.track_a,
                to_track: self.track_b.opposite(),
            },
            ConnectionDirection::Backward => DirectedTrackConnection {
                from_track: self.track_b,
                to_track: self.track_a.opposite(),
            },
        }
    }

    pub fn directed_connections(&self) -> [DirectedTrackConnection; 2] {
        [
            self.to_directed(ConnectionDirection::Forward),
            self.to_directed(ConnectionDirection::Backward),
        ]
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct DirectedTrackConnection {
    pub from_track: DirectedTrackID,
    pub to_track: DirectedTrackID,
}

impl DirectedTrackConnection {
    pub fn is_continuous(&self) -> bool {
        self.from_track.to_slot() == self.to_track.from_slot()
    }

    pub fn draw_with_gizmos(&self, gizmos: &mut Gizmos, scale: f32, color: Color) {
        let start = self.from_track.get_center_vec2() + self.from_track.get_delta_vec() * 0.2;
        let end = self.to_track.get_center_vec2() - self.to_track.get_delta_vec() * 0.2;
        gizmos.line_2d(start * scale, end * scale, color);
    }
}

#[derive(Clone, Copy)]
enum Turn {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Facing {
    Forward,
    Backward,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogicalTrackID {
    dirtrack: DirectedTrackID,
    facing: Facing,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TrackDirection {
    Aligned,
    Misaligned,
}

impl TrackDirection {
    pub fn opposite(&self) -> Self {
        match self {
            TrackDirection::Aligned => TrackDirection::Misaligned,
            TrackDirection::Misaligned => TrackDirection::Aligned,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DirectedTrackID {
    track: TrackID,
    direction: TrackDirection,
}

impl DirectedTrackID {
    pub fn opposite(&self) -> Self {
        Self {
            track: self.track,
            direction: self.direction.opposite(),
        }
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
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
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

    pub fn get_connection_to(&self, other: TrackID) -> Option<TrackConnection> {
        let cardinal = self.cell.cardinal_to(&other.cell)?;
        let track1 = self.get_directed_to_cardinal(cardinal)?;
        let track2 = other.get_directed_to_cardinal(cardinal.opposite())?;
        Some(TrackConnection {
            track_a: track1,
            track_b: track2,
        })
    }

    pub fn dirtracks(&self) -> [DirectedTrackID; 2] {
        [
            self.get_directed(TrackDirection::Aligned),
            self.get_directed(TrackDirection::Misaligned),
        ]
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
}
