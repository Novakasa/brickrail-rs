use bevy::prelude::*;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone, Copy, PartialEq)]
struct CellCoord {
    x: i32,
    y: i32,
    l: i32,
}

impl CellCoord {
    fn cardinal_to(&self, other: &Self) -> Option<Cardinal> {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dl = other.l - self.l;
        match (dx, dy, dl) {
            (0, 1, 0) => Some(Cardinal::N),
            (0, -1, 0) => Some(Cardinal::S),
            (1, 0, 0) => Some(Cardinal::E),
            (-1, 0, 0) => Some(Cardinal::W),
            _ => None,
        }
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
        match cardinal {
            Cardinal::N => Slot {
                cell: *self,
                interface: CellInterface::N,
            },
            Cardinal::E => Slot {
                cell: *self,
                interface: CellInterface::E,
            },
            Cardinal::S => Slot {
                cell: self.get_cardinal_neighbor(Cardinal::S),
                interface: CellInterface::N,
            },
            Cardinal::W => Slot {
                cell: self.get_cardinal_neighbor(Cardinal::W),
                interface: CellInterface::E,
            },
        }
    }

    fn get_cardinal_neighbor(&self, cardinal: Cardinal) -> Self {
        match cardinal {
            Cardinal::N => CellCoord {
                x: self.x,
                y: self.y + 1,
                l: self.l,
            },
            Cardinal::S => CellCoord {
                x: self.x,
                y: self.y - 1,
                l: self.l,
            },
            Cardinal::E => CellCoord {
                x: self.x + 1,
                y: self.y,
                l: self.l,
            },
            Cardinal::W => CellCoord {
                x: self.x - 1,
                y: self.y,
                l: self.l,
            },
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
    fn opposite(&self) -> Self {
        match self {
            Cardinal::N => Cardinal::S,
            Cardinal::S => Cardinal::N,
            Cardinal::E => Cardinal::W,
            Cardinal::W => Cardinal::E,
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
    }
}
