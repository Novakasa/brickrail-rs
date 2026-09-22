//! The client-only binary has no simulation behind it, so `LocalLayoutPlugin` is the
//! only thing putting layout elements in its world. If it stops spawning, that binary
//! silently renders nothing.

use bevy::prelude::*;
use brickrail_client::{LocalLayoutPlugin, test_layout};
use brickrail_common::lifecycle::ElementId;
use brickrail_common::track::Track;
use brickrail_common::train::Train;

#[test]
fn local_layout_spawns_elements_without_a_simulation() {
    let layout = test_layout();
    let expected_tracks = layout.tracks.len();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(brickrail_common::layout::LayoutAppPlugin);
    app.add_plugins(LocalLayoutPlugin(layout));

    // Startup writes the spawn messages; the lifecycle systems consume them next tick.
    app.update();
    app.update();

    let tracks = app
        .world_mut()
        .query::<&ElementId<Track>>()
        .iter(app.world())
        .count();
    assert_eq!(tracks, expected_tracks);

    let trains = app
        .world_mut()
        .query::<&ElementId<Train>>()
        .iter(app.world())
        .count();
    assert_eq!(trains, 1);
}
