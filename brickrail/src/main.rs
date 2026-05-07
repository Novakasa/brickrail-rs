use std::path::Path;

use bevy::{prelude::*, render::diagnostic::RenderDiagnosticsPlugin};
use bevy_inspector_egui::{DefaultInspectorConfigPlugin, bevy_egui};
use bevy_prototype_lyon::plugin::ShapePlugin;

use brickrail_rs::*;

fn main() {
    let file = Path::new("pybricks/programs/mpy/layout_controller.mpy");
    let hash = utils::get_file_hash(file);
    println!("Hash: {}", hash);
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window::default()),
            close_when_requested: false,
            ..Default::default()
        }))
        .add_plugins(ShapePlugin)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(editor::EditorPlugin)
        .add_plugins(persistent_hub_state::SettingsPlugin)
        .add_plugins(layout::LayoutPlugin)
        .add_plugins(block::BlockPlugin)
        .add_plugins(track::TrackPlugin)
        .add_plugins(train::TrainPlugin)
        .add_plugins(marker::MarkerPlugin)
        .add_plugins(crossing::CrossingPlugin)
        .add_plugins(DefaultInspectorConfigPlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugins(ble::BLEPlugin)
        .add_plugins(ble_train::BLETrainPlugin)
        .add_plugins(switch::SwitchPlugin)
        .add_plugins(switch_motor::PulseMotorPlugin)
        .add_plugins(layout_devices::LayoutDevicePlugin)
        .add_plugins(schedule::SchedulePlugin)
        .add_plugins(destination::DestinationPlugin)
        .add_plugins(RenderDiagnosticsPlugin::default())
        .add_plugins(materials::MaterialsPlugin)
        .add_plugins(route_modular::ModularRoutePlugin)
        .add_plugins(serial_broadcaster::SerialBroadcasterPlugin)
        .run();
}
