use crate::{
    block::Block,
    editor::*,
    layout::{Connections, EntityMap, MarkerMap, TrackLocks},
    layout_primitives::*,
    marker::{spawn_marker, Marker},
    route::{build_route, Route, TrainState},
    track::LAYOUT_SCALE,
};
use bevy::{input::keyboard, prelude::*, reflect::TypeRegistry};
use bevy_egui::egui::{self};
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::ShapePath,
    prelude::{LineCap, StrokeOptions},
    shapes::Line,
};
use bevy_trait_query::RegisterExt;

const TRAIN_WIDTH: f32 = 0.1;

#[derive(Resource, Default, Debug)]
struct TrainDragState {
    train_id: Option<TrainID>,
    target_dir: BlockDirection,
}

#[derive(Component, Debug)]
struct TrainWagon {
    id: WagonID,
}

#[derive(Bundle)]
struct TrainWagonBundle {
    wagon: TrainWagon,
    shape: ShapeBundle,
    stroke: Stroke,
}

impl TrainWagonBundle {
    fn new(id: WagonID) -> Self {
        let path = ShapePath::new()
            .add(&Line(Vec2::ZERO, Vec2::X * 0.5 * LAYOUT_SCALE))
            .build();
        let stroke = Stroke {
            color: Color::YELLOW,
            options: StrokeOptions::default()
                .with_line_width(TRAIN_WIDTH)
                .with_line_cap(LineCap::Round),
        };
        let shape = ShapeBundle {
            spatial: SpatialBundle::default(),
            path: path,
            ..default()
        };
        Self {
            wagon: TrainWagon { id },
            shape: shape,
            stroke: stroke,
        }
    }
}

#[derive(Debug, Reflect, Clone)]
struct TrainSettings {
    num_wagons: usize,
    home: Option<LogicalBlockID>,
    prefer_facing: Option<Facing>,
}

#[derive(Component, Debug, Clone)]
struct Train {
    id: TrainID,
    route: Option<Route>,
    state: TrainState,
    speed: f32,
    settings: TrainSettings,
}

impl Train {
    pub fn get_logical_block_id(&self) -> LogicalBlockID {
        self.route
            .as_ref()
            .unwrap()
            .get_current_leg()
            .get_target_block_id()
    }

    pub fn get_route(&self) -> &Route {
        self.route.as_ref().unwrap()
    }

    pub fn get_route_mut(&mut self) -> &mut Route {
        self.route.as_mut().unwrap()
    }

    fn traverse_route(&mut self, delta: f32) -> bool {
        let dist = delta * self.speed;
        let change_locks = self.get_route_mut().advance_distance(dist);
        self.state = self.get_route().get_train_state();
        self.speed = self.state.get_speed();
        return change_locks;
        // println!("Train state: {:?}, {:?}", self.state, self.speed);
        // println!("Route: {:?}", self.route.get_current_leg().section_position);
    }
}

impl Selectable for Train {
    fn inspector_ui(
        &mut self,
        ui: &mut egui::Ui,
        type_registry: &TypeRegistry,
        _entity_map: &mut EntityMap,
    ) {
        ui.label("Inspectable train lol");
        if ui.button("Turn around").clicked() {
            println!("can't lol");
        }
        ui_for_value(&mut self.settings, ui, type_registry);
    }

    fn get_id(&self) -> GenericID {
        GenericID::Train(self.id)
    }

    fn get_depth(&self) -> f32 {
        3.0
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.get_route()
            .get_current_leg()
            .get_current_pos()
            .distance(pos)
            - 0.2
    }
}

#[derive(Bundle)]
struct TrainBundle {
    train: Train,
}

impl TrainBundle {
    fn from_train(train: Train) -> Self {
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

        let pos = train.get_route().get_current_leg().get_current_pos();
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.2 * LAYOUT_SCALE, color);
    }
}

fn draw_train_route(mut gizmos: Gizmos, q_trains: Query<&Train>) {
    for train in q_trains.iter() {
        train.get_route().draw_with_gizmos(&mut gizmos);
    }
}

fn draw_locked_tracks(mut gizmos: Gizmos, track_locks: Res<TrackLocks>) {
    for (track, _) in track_locks.locked_tracks.iter() {
        for dirtrack in track.dirtracks() {
            dirtrack.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::RED);
        }
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
    entity_map: Res<EntityMap>,
    marker_map: Res<MarkerMap>,
    connections: Res<Connections>,
    mut track_locks: ResMut<TrackLocks>,
    mut q_trains: Query<&mut Train>,
    q_blocks: Query<&Block>,
    q_markers: Query<&Marker>,
) {
    if mouse_buttons.just_released(MouseButton::Right) {
        if let Some(train_id) = train_drag_state.train_id {
            if let Some(GenericID::Block(block_id)) = hover_state.hover {
                let mut train = q_trains
                    .get_mut(entity_map.get_entity(&GenericID::Train(train_id)).unwrap())
                    .unwrap();
                // println!("Dropping train {:?} on block {:?}", train_id, block_id);
                let start = train.get_logical_block_id();
                let target = block_id.to_logical(train_drag_state.target_dir, Facing::Forward);
                // println!("Start: {:?}, Target: {:?}", start, target);
                if let Some(logical_section) = connections.find_route_section(
                    start,
                    target,
                    Some((&train_id, &track_locks)),
                    train.settings.prefer_facing,
                ) {
                    // println!("Section: {:?}", section);
                    let route = build_route(
                        train_id,
                        &logical_section,
                        &q_markers,
                        &q_blocks,
                        &entity_map,
                        &marker_map,
                    );
                    // route.get_current_leg_mut().intention = LegIntention::Stop;
                    train.route = Some(route);
                    train.get_route().update_locks(&mut track_locks);
                    // println!("state: {:?}", train.route.get_train_state());
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

#[derive(Event)]
struct SpawnTrain {
    train: Train,
    block_id: LogicalBlockID,
}

fn create_train(
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    mut train_events: EventWriter<SpawnTrain>,
    entity_map: Res<EntityMap>,
    selection_state: Res<SelectionState>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::T) {
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            // println!("Creating train at block {:?}", block_id);
            let logical_block_id = block_id.to_logical(BlockDirection::Aligned, Facing::Forward);
            let train = Train {
                id: TrainID::new(entity_map.trains.len()),
                route: None,
                state: TrainState::Stop,
                speed: 0.0,
                settings: TrainSettings {
                    num_wagons: 3,
                    home: None,
                    prefer_facing: None,
                },
            };
            train_events.send(SpawnTrain {
                train: train,
                block_id: logical_block_id,
            });
        }
    }
}
fn spawn_train(
    mut train_events: EventReader<SpawnTrain>,
    mut commands: Commands,
    q_blocks: Query<&Block>,
    mut track_locks: ResMut<TrackLocks>,
    mut entity_map: ResMut<EntityMap>,
    marker_map: Res<MarkerMap>,
    q_markers: Query<&Marker>,
) {
    for request in train_events.read() {
        let mut train = request.train.clone();
        let block_id = request.block_id.clone();
        let block = q_blocks
            .get(
                entity_map
                    .get_entity(&GenericID::Block(block_id.block))
                    .unwrap(),
            )
            .unwrap();
        let block_section = block.get_logical_section(block_id);
        let train_id = TrainID::new(entity_map.trains.len());
        let route = build_route(
            train_id,
            &block_section,
            &q_markers,
            &q_blocks,
            &entity_map,
            &marker_map,
        );
        route.update_locks(&mut track_locks);
        train.route = Some(route);
        let train = TrainBundle::from_train(train);
        let train_id = train.train.id;
        // println!("Section: {:?}", block_section);
        // println!("Layout markers: {:?}", entity_map.markers);
        /*println!(
            "Creating train {:?} at logical block {:?}",
            train_id, logical_block_id
        );*/
        let entity = commands.spawn(train).id();
        entity_map.add_train(train_id, entity);
    }
}

fn update_train(
    mut q_trains: Query<&mut Train>,
    time: Res<Time>,
    mut track_locks: ResMut<TrackLocks>,
) {
    for mut train in q_trains.iter_mut() {
        if !track_locks.is_clean(&train.id) {
            // println!("Updating intentions for train {:?}", train.id);
            train
                .get_route_mut()
                .update_intentions(track_locks.as_ref());
            track_locks.mark_clean(&train.id);
        }
        let change_locks = train.traverse_route(time.delta_seconds());
        if change_locks {
            train.get_route().update_locks(&mut track_locks);
        }
    }
}

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Selectable, Train>();
        app.register_type::<Facing>();
        app.insert_resource(TrainDragState::default());
        app.add_event::<SpawnTrain>();
        app.add_systems(
            Update,
            (
                create_train,
                draw_train,
                draw_train_route,
                draw_locked_tracks.after(draw_train_route),
                init_drag_train,
                exit_drag_train,
                update_drag_train,
                update_train,
            ),
        );
        app.add_systems(
            PostUpdate,
            spawn_train
                .run_if(on_event::<SpawnTrain>())
                .after(spawn_marker),
        );
    }
}
