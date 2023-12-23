use crate::{
    block::Block,
    editor::*,
    layout::Layout,
    layout_primitives::*,
    marker::Marker,
    route::{build_route, Route},
    track::LAYOUT_SCALE,
};
use bevy::{input::keyboard, prelude::*, reflect::TypeRegistry};
use bevy_egui::egui;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_trait_query::RegisterExt;

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

impl Selectable for Train {
    fn inspector_ui(&mut self, ui: &mut egui::Ui, _type_registry: &TypeRegistry) {
        ui.label("Inspectable train lol");
    }

    fn get_id(&self) -> GenericID {
        GenericID::Train(self.id)
    }

    fn get_depth(&self) -> f32 {
        3.0
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.route.get_current_leg().get_current_pos().distance(pos) - 0.2
    }
}

#[derive(Bundle)]
struct TrainBundle {
    train: Train,
}

impl TrainBundle {
    fn new(route: Route, id: TrainID) -> Self {
        let route = route;
        let train = Train {
            id: id,
            route: route,
            wagons: vec![],
        };
        Self { train: train }
    }
}

fn draw_train(
    mut gizmos: Gizmos,
    q_trains: Query<&Train>,
    selection_state: Res<SelectionState>,
    hover_state: Res<HoverState>,
) {
    for train in q_trains.iter() {
        let mut color = Color::YELLOW;
        if hover_state.hover == Some(GenericID::Train(train.id)) {
            color = Color::RED;
        }
        if Selection::Single(GenericID::Train(train.id)) == selection_state.selection {
            color = Color::BLUE;
        }

        let pos = train.route.get_current_leg().get_current_pos();
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.2 * LAYOUT_SCALE, color);
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
        app.register_component_as::<dyn Selectable, Train>();
        app.add_systems(Update, (create_train, draw_train));
    }
}
