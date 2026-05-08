use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::connection::Connection;
use crate::lifecycle::{ElementEntry, LayoutType};
use crate::marker::Marker;
use crate::track::Track;
use crate::train::Train;

/// Serializable layout format. Handed from client to server when entering control mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Layout {
    pub tracks: Vec<ElementEntry<Track>>,
    /// All physical connections (both adjacent and portal).
    #[serde(default)]
    pub connections: Vec<ElementEntry<Connection>>,
    #[serde(default)]
    pub markers: Vec<ElementEntry<Marker>>,
    #[serde(default)]
    pub blocks: Vec<ElementEntry<Block>>,
    #[serde(default)]
    pub trains: Vec<ElementEntry<Train>>,
}

/// Marker component for the server-side layout instance.
#[derive(Component, Default)]
pub struct ServerLayout;

impl LayoutType for ServerLayout {}

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
