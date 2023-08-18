use bevy::prelude::*;

mod layout_primitives;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_framepace::FramepacePlugin)
        .run();
}
