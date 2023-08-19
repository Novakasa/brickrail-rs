use bevy::prelude::*;

mod layout;
mod layout_primitives;

use bevy_pancam::{PanCam, PanCamPlugin};
use layout::LayoutPlugin;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), PanCam::default()));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(PanCamPlugin::default())
        .add_plugins(LayoutPlugin {})
        .add_systems(Startup, spawn_camera)
        .run();
}
