use std::marker::PhantomData;

use bevy::prelude::*;

use crate::lifecycle::LayoutType;
use crate::route::RouteStatePlugin;

/// Top-level simulation state plugin. Both server and client include this.
/// Internally adds domain-specific sub-plugins for organizational purposes.
/// The app adds this one plugin — never the sub-plugins directly.
pub struct SimulationStatePlugin<L: LayoutType>(PhantomData<L>);

impl<L: LayoutType> Default for SimulationStatePlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: LayoutType> SimulationStatePlugin<L> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L: LayoutType> Plugin for SimulationStatePlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_plugins(RouteStatePlugin::<L>::new());
        // Future: app.add_plugins(LockStatePlugin::<L>::new());
        // Future: app.add_plugins(TrainSimStatePlugin::<L>::new());
    }
}
