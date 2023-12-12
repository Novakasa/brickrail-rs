use bevy::prelude::*;

mod block;
mod editor;
mod layout;
mod layout_primitives;
mod marker;
mod route;
mod section;
mod utils;

use block::BlockPlugin;
use editor::EditorPlugin;
use layout::LayoutPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(EditorPlugin {})
        .add_plugins(LayoutPlugin {})
        .add_plugins(BlockPlugin {})
        .run();
}
