use bevy::prelude::*;

mod editor;
mod layout;
mod layout_primitives;
mod utils;

use editor::EditorPlugin;
use layout::LayoutPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(EditorPlugin {})
        .add_plugins(LayoutPlugin {})
        .run();
}
