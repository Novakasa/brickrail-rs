//! Client-only binary: the view, with no simulation in this process.
//!
//! Scaffolding. The layout is spawned locally so there is something to draw; once a
//! network transport exists, `LocalLayoutPlugin` gives way to a connection to
//! `brickrail-server`, which owns the simulation.

use bevy::prelude::*;
use bevy_plugin_graph::{AddOwned, PluginGraphPlugin};
use brickrail_client::{ClientPlugin, LocalLayoutPlugin, test_layout};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(PluginGraphPlugin::new("ClientOnly"))
        .add_owned(ClientPlugin)
        .add_owned(LocalLayoutPlugin(test_layout()));

    if brickrail_common::dumped_plugin_graph(&mut app) {
        return;
    }

    app.run();
}
