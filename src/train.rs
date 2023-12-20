use crate::{block::Block, editor::*, layout::Layout, layout_primitives::*, route::Route};
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
    fn from_logical_block(logical_block_id: &LogicalBlockID, id: TrainID) -> Self {
        let route = Route::from_block(logical_block_id);
        let train = Train {
            id: id,
            route: route,
            home: logical_block_id.clone(),
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
    q_blocks: Query<&Block>,
    mut layout: ResMut<Layout>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::T) {
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            let logical_block_id = block_id.to_logical(BlockDirection::Aligned, Facing::Forward);
            let train_id = TrainID::new(layout.trains.len());
            let train = TrainBundle::from_logical_block(&logical_block_id, train_id);
            let train_id = train.train.id;
            println!("Creating train {:?}", train_id);
            let entity = commands.spawn(train).id();
            layout.add_train(train_id, entity);
        }
    }
}

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, create_train);
    }
}
