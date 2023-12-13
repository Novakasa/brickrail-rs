use crate::{
    block::{Block, LogicalBlock},
    editor::*,
    layout_primitives::*,
    route::Route,
};
use bevy::{input::keyboard, prelude::*};
use bevy_prototype_lyon::entity::ShapeBundle;

#[derive(Component, Debug)]
struct TrainWagon {
    id: TrainID,
    index: usize,
}

#[derive(Bundle)]
struct TrainWagonBundle {
    wagon: TrainWagon,
    transform: Transform,
    shape: ShapeBundle,
}

#[derive(Component, Debug)]
struct Train {
    id: TrainID,
    route: Route,
    home: LogicalBlockID,
    wagons: Vec<Entity>,
}

#[derive(Bundle)]
struct TrainBundle {
    train: Train,
    selectable: Selectable,
}

impl TrainBundle {
    fn from_logical_block(logical_block: &LogicalBlock) -> Self {
        let mut route = Route::from_block(logical_block);
        let train = Train {
            id: TrainID::new(logical_block.id),
            route: route,
            home: logical_block.id,
            wagons: vec![],
        };
        Self {
            selectable: Selectable::new(GenericID::Train(train.id), 0.0),
            train: train,
        }
    }
}

fn create_train(
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    mut commands: Commands,
    selection_state: Res<SelectionState>,
    q_logical_blocks: Query<&LogicalBlock>,
    q_blocks: Query<&Block>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::T) {
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            let logical_block_id = block_id.to_logical(BlockDirection::Aligned, Facing::Forward);
            // let logical_block =
            // let train = TrainBundle::from_logical_block(logical_block);
            // commands.spawn(train);
        }
    }
}

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {}
}
