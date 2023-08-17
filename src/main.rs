use bevy::prelude::*;

#[derive(Clone, Copy)]
struct GridCoord {
    x: i32,
    y: i32,
    l: i32,
}

#[derive(Clone, Copy)]
enum Slot {
    N,
    S,
    E,
    W,
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
    fn from_slots(slot1: Slot, slot2: Slot) -> Self {
        match (slot1, slot2) {
            (Slot::N, Slot::S) => Orientation::NS,
            (Slot::N, Slot::E) => Orientation::NE,
            (Slot::N, Slot::W) => Orientation::NW,
            (Slot::S, Slot::E) => Orientation::SE,
            (Slot::S, Slot::W) => Orientation::SW,
            (Slot::E, Slot::W) => Orientation::EW,
            _ => Self::from_slots(slot2, slot1),
        }
    }
}

struct TrackConnection {
    track1: Track,
    track2: Track,
}

#[derive(Clone, Copy)]
struct Track {
    coords: GridCoord,
    orientation: Orientation,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .run();
}
