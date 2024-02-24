use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    editor::{GenericID, Selectable, SpawnEvent},
    layout_primitives::*,
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Debug, Reflect, Serialize, Deserialize, Clone)]
enum SwitchPosition {
    Left,
    Center,
    Right,
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
struct Switch {
    id: DirectedTrackID,
    positions: Vec<SwitchPosition>,
    pos_index: usize,
}

impl Selectable for Switch {
    fn get_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.id.distance_to(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}
pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEvent<Switch>>();
    }
}
