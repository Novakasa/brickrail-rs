use crate::layout_primitives::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Serialize, Deserialize, Clone)]
struct SwitchMotor {
    hub_id: HubID,
    port: HubPort,
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct BLESwitch {
    id: DirectedTrackID,
    motors: Vec<SwitchMotor>,
}

impl BLESwitch {
    pub fn new(id: DirectedTrackID) -> Self {
        Self {
            id,
            motors: Vec::new(),
        }
    }
}

struct BLESwitchPlugin;

impl Plugin for BLESwitchPlugin {
    fn build(&self, app: &mut App) {}
}
