use crate::{layout_primitives::*, route::Route};
use bevy::prelude::*;

#[derive(Component, Debug)]
struct Train {
    id: TrainID,
    route: Route,
    home: LogicalBlockID,
}

struct TrainBundle {
    train: Train,
    transform: Transform,
}

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {}
}
