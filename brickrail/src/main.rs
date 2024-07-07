use std::{env, path::Path};

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui;

mod bevy_tokio_tasks;
mod ble;
mod ble_train;
mod block;
mod editor;
mod inspector;
mod layout;
mod layout_devices;
mod layout_primitives;
mod marker;
mod route;
mod schedule;
mod section;
mod settings;
mod switch;
mod switch_motor;
mod track;
mod train;
mod utils;

fn main() {
    let file = Path::new("pybricks/programs/mpy/layout_controller.mpy");
    let hash = utils::get_file_hash(file);
    println!("Hash: {}", hash);
    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LOG", "pybricks_ble=info,brickrail=info");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window::default()),
            close_when_requested: false,
            ..Default::default()
        }))
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(editor::EditorPlugin)
        .add_plugins(settings::SettingsPlugin)
        .add_plugins(layout::LayoutPlugin)
        .add_plugins(block::BlockPlugin)
        .add_plugins(track::TrackPlugin)
        .add_plugins(train::TrainPlugin)
        .add_plugins(marker::MarkerPlugin)
        .add_plugins(inspector::InspectorPlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugins(ble::BLEPlugin)
        .add_plugins(ble_train::BLETrainPlugin)
        .add_plugins(switch::SwitchPlugin)
        .add_plugins(switch_motor::SwitchMotorPlugin)
        .add_plugins(layout_devices::LayoutDevicePlugin)
        .add_plugins(schedule::SchedulePlugin)
        .run();
}
