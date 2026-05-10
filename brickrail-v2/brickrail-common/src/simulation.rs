use bevy::prelude::*;

use crate::route::RouteStatePlugin;
use crate::train_position::TrainPositionStatePlugin;

/// Top-level simulation state plugin. Both server and client include this.
/// Internally adds domain-specific sub-plugins for organizational purposes.
/// The app adds this one plugin — never the sub-plugins directly.
pub struct SimulationStatePlugin;

impl Plugin for SimulationStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RouteStatePlugin);
        app.add_plugins(TrainPositionStatePlugin);
        // Future: app.add_plugins(LockStatePlugin);
    }
}
