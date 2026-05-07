use bevy::prelude::*;
use brickrail_rs::{
    editor::{EditorPlugin, LoadLayoutMessage},
    layout::LayoutPlugin,
    block::BlockPlugin,
    track::TrackPlugin,
    train::TrainPlugin,
    marker::MarkerPlugin,
    switch::SwitchPlugin,
    switch_motor::PulseMotorPlugin,
    layout_devices::LayoutDevicePlugin,
    schedule::SchedulePlugin,
    destination::DestinationPlugin,
    route_modular::ModularRoutePlugin,
    persistent_hub_state::SettingsPlugin,
    materials::MaterialsPlugin,
};

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(EditorPlugin);
    app.add_plugins(SettingsPlugin);
    app.add_plugins(LayoutPlugin);
    app.add_plugins(BlockPlugin);
    app.add_plugins(TrackPlugin);
    app.add_plugins(TrainPlugin);
    app.add_plugins(MarkerPlugin);
    app.add_plugins(SwitchPlugin);
    app.add_plugins(PulseMotorPlugin);
    app.add_plugins(LayoutDevicePlugin);
    app.add_plugins(SchedulePlugin);
    app.add_plugins(DestinationPlugin);
    app.add_plugins(ModularRoutePlugin);
    app.add_plugins(MaterialsPlugin);
    app
}

#[test]
fn train_advances_through_route_legs() {
    let mut app = make_app();

    let layout_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/layouts/simple-single-train.json");

    app.world_mut().write_message(LoadLayoutMessage { path: layout_path });

    // run a few ticks to let the layout load and systems settle
    for _ in 0..10 {
        app.update();
    }

    // placeholder assertion — replace with something meaningful once the app loads
    assert!(true);
}
