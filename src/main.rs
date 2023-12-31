use std::env;

use bevy::prelude::*;

mod block;
mod editor;
mod inspector;
mod layout;
mod layout_primitives;
mod marker;
mod route;
mod section;
mod track;
mod train;
mod utils;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(editor::EditorPlugin)
        .add_plugins(layout::LayoutPlugin)
        .add_plugins(block::BlockPlugin)
        .add_plugins(track::TrackPlugin)
        .add_plugins(train::TrainPlugin)
        .add_plugins(inspector::InspectorPlugin)
        .run();
}
