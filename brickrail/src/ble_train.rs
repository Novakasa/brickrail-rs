use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::layout_primitives::HubID;

#[derive(Component, Serialize, Deserialize)]
struct BLETrain {
    master_hub: HubID,
    puppets: Vec<HubID>,
}
