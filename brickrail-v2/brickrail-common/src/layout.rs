use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::track::Track;

/// Serializable layout format. Handed from client to server when entering control mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Layout {
    pub tracks: Vec<Track>,
}

/// Command: enter control mode with a serialized layout.
#[derive(Message, Clone)]
pub struct EnterControlMode {
    pub layout: Layout,
}

/// Command: exit control mode.
#[derive(Message, Clone)]
pub struct ExitControlMode;

/// Server state machine.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ServerState {
    #[default]
    Idle,
    Running,
}
