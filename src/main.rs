use bevy::prelude::*;

mod block;
mod editor;
mod layout;
mod layout_primitives;
mod marker;
mod route;
mod section;
mod track;
mod utils;

use block::BlockPlugin;
use editor::EditorPlugin;
use layout::LayoutPlugin;
use track::TrackPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(EditorPlugin {})
        .add_plugins(LayoutPlugin {})
        .add_plugins(BlockPlugin {})
        .add_plugins(TrackPlugin {})
        .run();
}
