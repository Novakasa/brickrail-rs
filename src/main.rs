#![allow(dead_code)]
use bevy::prelude::*;
use strum_macros::EnumIter;

#[derive(Clone, Copy, PartialEq)]
struct CellCoord {
    x: i32,
    y: i32,
    l: i32,
}

impl CellCoord {
    fn cardinal_to(&self, other: &Self) -> Option<Cardinal> {
        Some(Cardinal::from_deltas(other.x - self.x, other.y - self.y)?)
    }

    fn cardinal_to_slot(&self, slot: &Slot) -> Option<Cardinal> {
        if self == &slot.cell {
            return Some(slot.interface.to_cardinal());
        }
        if self == &slot.get_other_cell() {
            return Some(slot.interface.to_cardinal().opposite());
        }
        None
    }

    fn get_slot(&self, cardinal: Cardinal) -> Slot {
        if let Some(interface) = CellInterface::from_cardinal(cardinal) {
            return Slot {
                cell: *self,
                interface,
            };
        }
        return Slot {
            cell: self.get_cardinal_neighbor(cardinal),
            interface: CellInterface::from_cardinal(cardinal.opposite()).unwrap(),
        };
    }

    fn get_cardinal_neighbor(&self, cardinal: Cardinal) -> Self {
        Self {
            x: self.x + cardinal.dx(),
            y: self.y + cardinal.dy(),
            l: self.l,
        }
    }

    fn get_shared_slot(&self, other: &Self) -> Option<Slot> {
        if let Some(cardinal) = self.cardinal_to(other) {
            return Some(self.get_slot(cardinal));
        }
        None
    }
}

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone, Copy)]
struct Slot {
    cell: CellCoord,
    interface: CellInterface,
}

impl Slot {
    fn can_connect_to(&self, other: &Self) -> bool {
        if self.cell == other.cell {
            println!("same cell");
            return self.interface != other.interface;
        }
        if self.get_other_cell() == other.cell {
            println!("other cell is neighbor");
            return self.interface.to_cardinal() != other.interface.to_cardinal().opposite();
        }
        if self.cell == other.get_other_cell() {
            println!("self cell is neighbor");
            return self.interface.to_cardinal().opposite() != other.interface.to_cardinal();
        }
        println!("other cell is not neighbor");
        return false;
    }

    fn get_shared_cell(&self, other: &Self) -> Option<CellCoord> {
        if self.get_other_cell() == other.cell {
            return Some(self.cell);
        }
        if self.cell == other.get_other_cell() {
            return Some(other.cell);
        }
        None
    }

    fn get_other_cell(&self) -> CellCoord {
        self.cell
            .get_cardinal_neighbor(self.interface.to_cardinal())
    }
}

#[derive(Clone, Copy, PartialEq, EnumIter, Debug)]
enum Cardinal {
    N,
    S,
    E,
    W,
}

impl Cardinal {
    fn from_deltas(dx: i32, dy: i32) -> Option<Self> {
        match (dx, dy) {
            (0, 1) => Some(Cardinal::N),
            (0, -1) => Some(Cardinal::S),
            (1, 0) => Some(Cardinal::E),
            (-1, 0) => Some(Cardinal::W),
            _ => None,
        }
    }

    fn opposite(&self) -> Self {
        match self {
            Cardinal::N => Cardinal::S,
            Cardinal::S => Cardinal::N,
            Cardinal::E => Cardinal::W,
            Cardinal::W => Cardinal::E,
        }
    }

    fn dx(&self) -> i32 {
        match self {
            Cardinal::E => 1,
            Cardinal::W => -1,
            _ => 0,
        }
    }

    fn dy(&self) -> i32 {
        match self {
            Cardinal::N => 1,
            Cardinal::S => -1,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy)]
enum Orientation {
    NS,
    NE,
    NW,
    SE,
    SW,
    EW,
}

impl Orientation {
    fn from_cardinals(slot1: Cardinal, slot2: Cardinal) -> Self {
        match (&slot1, &slot2) {
            (Cardinal::N, Cardinal::S) => Orientation::NS,
            (Cardinal::N, Cardinal::E) => Orientation::NE,
            (Cardinal::N, Cardinal::W) => Orientation::NW,
            (Cardinal::S, Cardinal::E) => Orientation::SE,
            (Cardinal::S, Cardinal::W) => Orientation::SW,
            (Cardinal::E, Cardinal::W) => Orientation::EW,
            _ => Self::from_cardinals(slot2, slot1),
        }
    }
}

struct TrackConnection {
    track1: DirectedTrack,
    track2: DirectedTrack,
}

impl TrackConnection {}

struct DirectedTrack {
    from_slot: Slot,
    to_slot: Slot,
}

impl DirectedTrack {
    fn opposite(&self) -> Self {
        Self {
            from_slot: self.to_slot,
            to_slot: self.from_slot,
        }
    }
}

#[derive(Clone, Copy)]
struct Track {
    cell: CellCoord,
    orientation: Orientation,
}

impl Track {
    fn from_slots(slot1: Slot, slot2: Slot) -> Option<Self> {
        let cell = slot1.get_shared_cell(&slot2)?;
        let card1 = cell.cardinal_to(&slot1.cell)?;
        let card2 = cell.cardinal_to(&slot2.cell)?;
        let orientation = Orientation::from_cardinals(card1, card2);
        Some(Self { cell, orientation })
    }

    fn from_cells(cell1: CellCoord, cell2: CellCoord, cell3: CellCoord) -> Option<Self> {
        let slot0 = cell1.get_shared_slot(&cell2)?;
        let slot1 = cell2.get_shared_slot(&cell3)?;
        Self::from_slots(slot0, slot1)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .run();
}

#[cfg(test)]

mod test {

    #[test]
    fn test_slot_connectivity() {
        use super::*;
        use strum::IntoEnumIterator;

        let slot1 = Slot {
            cell: CellCoord { x: 0, y: 0, l: 0 },
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
            cell: CellCoord { x: 0, y: 0, l: 0 },
            interface: CellInterface::N,
        };

        let cell2 = CellCoord { x: 0, y: 2, l: 0 };

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
}
