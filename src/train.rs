use crate::{
    block::Block,
    editor::*,
    layout::Layout,
    layout_primitives::*,
    marker::Marker,
    route::{build_route, LegIntention, Route},
    track::LAYOUT_SCALE,
};
use bevy::{input::keyboard, prelude::*, reflect::TypeRegistry};
use bevy_egui::egui;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_trait_query::RegisterExt;

#[derive(Resource, Default, Debug)]
struct TrainDragState {
    train_id: Option<TrainID>,
    target_dir: BlockDirection,
}

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

impl Train {
    pub fn get_logical_block_id(&self) -> LogicalBlockID {
        self.route.get_current_leg().get_target_block_id()
    }
}

impl Selectable for Train {
    fn inspector_ui(&mut self, ui: &mut egui::Ui, _type_registry: &TypeRegistry) {
        ui.label("Inspectable train lol");
        if ui.button("Turn around").clicked() {
            println!("can't lol");
        }
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

fn draw_train_route(mut gizmos: Gizmos, q_trains: Query<&Train>) {
    for train in q_trains.iter() {
        train.route.draw_with_gizmos(&mut gizmos);
    }
}

fn init_drag_train(
    mouse_buttons: Res<Input<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    hover_state: Res<HoverState>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        if let Some(GenericID::Train(train_id)) = hover_state.hover {
            train_drag_state.train_id = Some(train_id);
            train_drag_state.target_dir = BlockDirection::Aligned;
            println!("Dragging train {:?}", train_id)
        }
    }
}

fn exit_drag_train(
    mouse_buttons: Res<Input<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    hover_state: Res<HoverState>,
    layout: Res<Layout>,
    mut q_trains: Query<&mut Train>,
    q_markers: Query<&Marker>,
) {
    if mouse_buttons.just_released(MouseButton::Right) {
        if let Some(train_id) = train_drag_state.train_id {
            let mut train = q_trains
                .get_mut(layout.get_entity(&GenericID::Train(train_id)).unwrap())
                .unwrap();
            if let Some(GenericID::Block(block_id)) = hover_state.hover {
                println!("Dropping train {:?} on block {:?}", train_id, block_id);
                let start = train.get_logical_block_id();
                let target = block_id.to_logical(train_drag_state.target_dir, Facing::Forward);
                println!("Start: {:?}, Target: {:?}", start, target);
                if let Some(section) = layout.find_route_section(start, target) {
                    println!("Section: {:?}", section);
                    let mut route = build_route(&section, &q_markers, &layout);
                    route.get_current_leg_mut().intention = LegIntention::Stop;
                    train.route = route;
                    println!("state: {:?}", train.route.get_train_state());
                }
            }
            train_drag_state.train_id = None;
        }
    }
}

fn update_drag_train(
    mouse_buttons: Res<Input<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
) {
    if train_drag_state.train_id.is_none() {
        return;
    }
    if mouse_buttons.just_pressed(MouseButton::Left) {
        train_drag_state.target_dir = train_drag_state.target_dir.opposite();
        println!("Target dir: {:?}", train_drag_state.target_dir)
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
        app.insert_resource(TrainDragState::default());
        app.add_systems(
            Update,
            (
                create_train,
                draw_train,
                draw_train_route,
                init_drag_train,
                exit_drag_train,
                update_drag_train,
            ),
        );
    }
}
