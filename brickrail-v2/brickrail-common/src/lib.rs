pub mod block;
pub mod command;
pub mod connection;
pub mod driver;
pub mod layout;
pub mod layout_primitives;
pub mod lifecycle;
pub mod logical_graph;
pub mod marker;
pub mod route;
pub mod simulation;
pub mod simulation_event;
pub mod track;
pub mod train;
pub mod train_position;
pub mod virtual_driver;

use bevy::prelude::App;

/// Handles the case where a binary was run only to dump its plugin graph.
///
/// `bevy_plugin_graph` writes from `App::finish`, so a graph can be produced without
/// ever starting the runner — no window, no game loop. Returns `true` when the graph
/// was written and the binary should exit instead of running.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # let mut app = App::new();
/// if brickrail_common::dumped_plugin_graph(&mut app) {
///     return;
/// }
/// app.run();
/// ```
pub fn dumped_plugin_graph(app: &mut App) -> bool {
    if std::env::var_os(bevy_plugin_graph::ENV_OUTPUT).is_none() {
        return false;
    }
    app.finish();
    true
}
