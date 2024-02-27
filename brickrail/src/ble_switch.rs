use crate::layout_primitives::*;
use bevy::prelude::*;

struct SwitchMotor {
    hub_id: HubID,
    port: HubPort,
}

struct BLESwitch {
    id: DirectedTrackID,
    motors: Vec<SwitchMotor>,
}

struct BLESwitchPlugin;

impl Plugin for BLESwitchPlugin {
    fn build(&self, app: &mut App) {}
}
