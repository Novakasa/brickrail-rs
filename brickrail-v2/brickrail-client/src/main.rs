//! Combined binary: client and simulation in a single process.
//!
//! The simulation runs in a SubApp, so there is no transport to configure and no
//! second process to start. Pair `client-only` with `brickrail-server` for the split
//! setup.

use bevy::prelude::*;
use bevy_plugin_graph::{AddOwned, PluginGraphPlugin};
use brickrail_client::{ClientPlugin, ClientSimulationPlugin, DemoScenarioPlugin};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(PluginGraphPlugin::new("Client"))
        .add_owned(ClientPlugin)
        .add_owned(ClientSimulationPlugin)
        .add_owned(DemoScenarioPlugin);

    if brickrail_common::dumped_plugin_graph(&mut app) {
        return;
    }

    app.run();
}
