use bevy::app::AppLabel;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::connection::Connection;
use crate::lifecycle::{ElementEntry, ElementPlugin};
use crate::logical_graph::LogicalGraphPlugin;
use crate::marker::Marker;
use crate::simulation::SimulationStatePlugin;
use crate::track::Track;
use crate::train::Train;

/// Label for the layout SubApp.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct LayoutSubApp;

/// Reusable plugin that adds all layout infrastructure.
/// Assumes standard Bevy schedules and message plumbing are already present.
pub struct LayoutAppPlugin;

impl Plugin for LayoutAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ElementPlugin::<Track>::new());
        app.add_plugins(ElementPlugin::<Connection>::new());
        app.add_plugins(ElementPlugin::<Marker>::new());
        app.add_plugins(ElementPlugin::<Block>::new());
        app.add_plugins(ElementPlugin::<Train>::new());
        app.add_plugins(LogicalGraphPlugin);
        app.add_plugins(SimulationStatePlugin);
    }
}

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
