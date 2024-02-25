use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    editor::{GenericID, Selectable, SpawnEvent},
    layout_primitives::*,
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Switch {
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

#[derive(Serialize, Deserialize, Clone)]
pub struct SerializedSwitch {
    pub switch: Switch,
}

#[derive(Debug, Event)]
pub struct UpdateSwitchTurnsEvent {
    pub id: DirectedTrackID,
    pub positions: Vec<SwitchPosition>,
}
pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEvent<SerializedSwitch>>();
        app.add_event::<UpdateSwitchTurnsEvent>();
    }
}
