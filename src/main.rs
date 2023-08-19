use bevy::prelude::*;

mod layout;
mod layout_primitives;

use layout::LayoutPlugin;

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(LayoutPlugin {})
        .add_systems(Startup, spawn_camera)
        .run();
}
