//! Standalone server binary: the simulation, headless.
//!
//! Scaffolding. `ServerPlugin` brings up the layout SubApp and its control-mode state
//! machine; what is still missing is the transport that lets a `client-only` process
//! send commands and receive `SimulationEvent`s. Until then this runs and idles.

use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_plugin_graph::{AddOwned, PluginGraphPlugin};
use brickrail_server::ServerPlugin;

/// Headless tick rate. Arbitrary until the transport dictates one.
const TICK: Duration = Duration::from_micros(16_667);

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(TICK)))
        .add_plugins(PluginGraphPlugin::new("Server"))
        .add_owned(ServerPlugin);

    if brickrail_common::dumped_plugin_graph(&mut app) {
        return;
    }

    app.run();
}
