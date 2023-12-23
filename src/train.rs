use crate::{
    block::Block,
    editor::*,
    layout::Layout,
    layout_primitives::*,
    marker::Marker,
    route::{build_route, Route},
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
    wagons: Vec<Entity>,
}

#[derive(Bundle)]
struct TrainBundle {
    train: Train,
    selectable: Selectable,
}

impl TrainBundle {
    fn new(route: Route, id: TrainID) -> Self {
        let route = route;
        let train = Train {
            id: id,
            route: route,
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
    q_markers: Query<&Marker>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::T) {
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            let logical_block_id = block_id.to_logical(BlockDirection::Aligned, Facing::Forward);
            let block = q_blocks
                .get(layout.get_entity(&GenericID::Block(*block_id)).unwrap())
                .unwrap();
            let block_section = block.get_logical_section(logical_block_id);
            let train_id = TrainID::new(layout.trains.len());
            let route = build_route(&block_section, &q_markers, &layout);
            let train = TrainBundle::new(route, train_id);
            let train_id = train.train.id;
            println!("Section: {:?}", block_section);
            println!("Layout markers: {:?}", layout.markers);
            println!("Layout in markers: {:?}", layout.in_markers);
            println!(
                "Creating train {:?} at logical block {:?}",
                train_id, logical_block_id
            );
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
