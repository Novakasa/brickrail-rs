use std::{env, path::Path};

use bevy::{prelude::*, render::diagnostic::RenderDiagnosticsPlugin};
use bevy_inspector_egui::bevy_egui;
use bevy_prototype_lyon::plugin::ShapePlugin;

mod bevy_tokio_tasks;
mod ble;
mod ble_train;
mod block;
mod crossing;
mod destination;
mod editor;
mod inspector;
mod layout;
mod layout_devices;
mod layout_primitives;
mod marker;
mod materials;
mod route;
mod schedule;
mod section;
mod selectable;
mod settings;
mod switch;
mod switch_motor;
mod track;
mod track_mesh;
mod train;
mod utils;

fn main() {
    let file = Path::new("pybricks/programs/mpy/layout_controller.mpy");
    let hash = utils::get_file_hash(file);
    println!("Hash: {}", hash);
    env::set_var("RUST_BACKTRACE", "1");
    env::set_var("RUST_LOG", "pybricks_ble=info,brickrail=info,bevy=info");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window::default()),
            close_when_requested: false,
            ..Default::default()
        }))
        .add_plugins(ShapePlugin)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(editor::EditorPlugin)
        .add_plugins(settings::SettingsPlugin)
        .add_plugins(layout::LayoutPlugin)
        .add_plugins(block::BlockPlugin)
        .add_plugins(track::TrackPlugin)
        .add_plugins(train::TrainPlugin)
        .add_plugins(marker::MarkerPlugin)
        .add_plugins(crossing::CrossingPlugin)
        .add_plugins(inspector::InspectorPlugin)
        .add_plugins(bevy_tokio_tasks::TokioTasksPlugin::default())
        .add_plugins(ble::BLEPlugin)
        .add_plugins(ble_train::BLETrainPlugin)
        .add_plugins(switch::SwitchPlugin)
        .add_plugins(switch_motor::PulseMotorPlugin)
        .add_plugins(layout_devices::LayoutDevicePlugin)
        .add_plugins(schedule::SchedulePlugin)
        .add_plugins(destination::DestinationPlugin)
        // .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(RenderDiagnosticsPlugin::default())
        .add_plugins(materials::MaterialsPlugin)
        .run();
}
