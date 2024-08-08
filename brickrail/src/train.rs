use crate::{
    ble::HubCommandEvent,
    ble_train::BLETrain,
    block::{spawn_block, Block},
    destination::{BlockDirectionFilter, Destination},
    editor::*,
    layout::{Connections, EntityMap, MarkerMap, TrackLocks},
    layout_primitives::*,
    marker::Marker,
    route::{build_route, LegState, Route, TrainState},
    schedule::{AssignedSchedule, ControlInfo, TrainSchedule},
    section::LogicalSection,
    switch::{SetSwitchPositionEvent, Switch},
    track::LAYOUT_SCALE,
};
use bevy::{
    color::palettes::css::{ORANGE, RED, YELLOW},
    ecs::system::{SystemParam, SystemState},
};
use bevy::{input::keyboard, prelude::*};
use bevy_egui::egui::Ui;
use bevy_inspector_egui::bevy_egui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_mouse_tracking_plugin::MousePosWorld;
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::ShapePath,
    prelude::{LineCap, StrokeOptions},
    shapes::Line,
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

const TRAIN_WIDTH: f32 = 0.3;
const WAGON_DIST: f32 = 0.7;
const WAGON_LENGTH: f32 = 0.6;

#[derive(Resource, Default, Debug)]
pub struct TrainDragState {
    train_id: Option<TrainID>,
    target: Option<LogicalBlockID>,
    target_facing: Facing,
    pub route: Option<Route>,
}

#[derive(Component, Debug)]
pub struct TrainWagon {
    pub id: WagonID,
}

impl Selectable for TrainWagon {
    type SpawnEvent = SpawnTrainEvent;
    type ID = WagonID;

    fn generic_id(&self) -> GenericID {
        GenericID::Train(self.id.train)
    }

    fn id(&self) -> WagonID {
        self.id
    }

    fn get_depth(&self) -> f32 {
        3.0
    }

    fn get_distance(
        &self,
        pos: Vec2,
        transform: Option<&Transform>,
        stroke: Option<&Stroke>,
    ) -> f32 {
        if stroke.unwrap().color.alpha() < 0.2 {
            return 30.0;
        }
        let transform = transform.unwrap();
        let pos_local = transform
            .compute_affine()
            .inverse()
            .transform_point(pos.extend(0.0) * LAYOUT_SCALE)
            .truncate()
            / LAYOUT_SCALE;

        let extent = Vec2::new(WAGON_DIST * 0.6, TRAIN_WIDTH * 0.5);
        let vec_to_closest_corner = pos_local.abs() - extent;

        vec_to_closest_corner.max(Vec2::ZERO).length()
            + vec_to_closest_corner
                .x
                .max(vec_to_closest_corner.y)
                .min(0.0)
    }
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
            .add(&Line(
                -Vec2::X * 0.5 * (WAGON_LENGTH - TRAIN_WIDTH) * LAYOUT_SCALE,
                Vec2::X * 0.5 * (WAGON_LENGTH - TRAIN_WIDTH) * LAYOUT_SCALE,
            ))
            .build();
        let stroke = Stroke {
            color: Color::from(YELLOW),
            options: StrokeOptions::default()
                .with_line_width(TRAIN_WIDTH * LAYOUT_SCALE)
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

#[derive(Debug, Reflect, Clone, Serialize, Deserialize)]
struct TrainSettings {
    num_wagons: usize,
    home: Option<LogicalBlockID>,
    prefer_facing: Option<Facing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Position")]
enum SerializablePosition {
    Block(LogicalBlockID),
    Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "SerializablePosition", into = "SerializablePosition")]
enum Position {
    Route(Route),
    Block(LogicalBlockID),
    Storage,
}

impl From<SerializablePosition> for Position {
    fn from(pos: SerializablePosition) -> Self {
        match pos {
            SerializablePosition::Block(block) => Position::Block(block),
            SerializablePosition::Storage => Position::Storage,
        }
    }
}

impl Into<SerializablePosition> for Position {
    fn into(self) -> SerializablePosition {
        match self {
            Position::Block(block) => SerializablePosition::Block(block),
            Position::Storage => SerializablePosition::Storage,
            Position::Route(route) => {
                SerializablePosition::Block(route.get_current_leg().get_target_block_id())
            }
        }
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Train {
    pub id: TrainID,
    position: Position,
    #[serde(skip)]
    state: TrainState,
    #[serde(skip)]
    speed: f32,
    #[serde(skip)]
    seek_speed: f32,
    #[serde(skip)]
    seek_pos: f32,
    #[serde(skip)]
    in_place_cycle: f32,
    settings: TrainSettings,
    #[serde(skip)]
    wagons: Vec<WagonID>,
}

impl Train {
    pub fn at_block_id(train_id: TrainID, logical_block_id: LogicalBlockID) -> Train {
        let train = Train {
            id: train_id,
            position: Position::Block(logical_block_id),
            state: TrainState::Stop,
            speed: 0.0,
            in_place_cycle: 0.0,
            seek_speed: 0.0,
            seek_pos: 0.0,
            settings: TrainSettings {
                num_wagons: 3,
                home: None,
                prefer_facing: None,
            },
            wagons: vec![],
        };
        train
    }

    pub fn get_logical_block_id(&self) -> LogicalBlockID {
        self.get_route().get_current_leg().get_target_block_id()
    }

    pub fn get_route(&self) -> &Route {
        match &self.position {
            Position::Route(route) => route,
            _ => panic!("Train {:?} has no route", self.id),
        }
    }

    pub fn get_route_mut(&mut self) -> &mut Route {
        match &mut self.position {
            Position::Route(route) => route,
            _ => panic!("Train {:?} has no route", self.id),
        }
    }

    pub fn advance_sensor(&mut self) {
        let route = self.get_route_mut();
        route.advance_sensor().expect("Failed to advance sensor");

        self.set_seek_target();
    }

    fn set_seek_target(&mut self) {
        let route = self.get_route();
        let current_pos = route.get_current_leg().get_signed_pos_from_first();
        let prev_marker_pos = route
            .get_current_leg()
            .get_prev_marker_signed_from_first(WAGON_DIST);

        self.seek_pos = prev_marker_pos - current_pos;
        // shift by how much the train will be out of phase after seeking
        // so seeking basically undoes the phase shift
        self.seek_pos -= (self.seek_pos + (1.0 - self.in_place_cycle) * WAGON_DIST) % WAGON_DIST;
    }

    fn traverse_route(&mut self, delta: f32, advance_events: &mut EventWriter<MarkerAdvanceEvent>) {
        let target_speed = self.state.get_speed();
        self.speed += ((target_speed - self.speed) * 2.8 - self.speed * 0.5) * delta;
        let dist = delta * self.speed;
        self.get_route_mut().advance_distance(dist, advance_events);
        self.state = self.get_route().get_train_state();
        // self.speed = self.state.get_speed();
        // println!("Train state: {:?}, {:?}", self.state, self.speed);
        // println!("Route: {:?}", self.route.get_current_leg().section_position);
    }

    fn traverse_route_passive(&mut self, delta: f32) {
        let target_speed = self.get_route().get_train_state().get_speed();
        self.speed += ((target_speed - self.speed) * 2.8 - self.speed * 0.5) * delta;

        let route = self.get_route_mut();
        let current_pos = route.get_current_leg().get_signed_pos_from_first();
        let mut move_mod = 1.0;

        let travel_sign = target_speed.signum();
        if let Some(next_marker_pos) = route
            .get_current_leg()
            .get_next_marker_signed_from_first(-0.2)
        {
            let dist = (next_marker_pos - current_pos) * travel_sign;
            move_mod = dist.clamp(0.0, WAGON_DIST) / WAGON_DIST;
        }

        self.seek_speed += (self.seek_pos * 40.0 - self.seek_speed * 10.0) * delta;
        let move_speed = self.speed * move_mod + self.seek_speed;

        self.in_place_cycle += delta * (self.speed - move_speed) / WAGON_DIST;
        self.in_place_cycle = self.in_place_cycle.rem_euclid(1.0);
        self.seek_pos -= self.seek_speed * delta;
        let new_pos = current_pos + move_speed * delta;
        self.get_route_mut()
            .get_current_leg_mut()
            .set_signed_pos_from_first(new_pos);
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<(&mut Train, Option<&mut AssignedSchedule>)>,
            Query<(&TrainSchedule, Option<&Name>)>,
            ResMut<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            Commands,
            Res<ControlInfo>,
        )>::new(world);
        let (
            mut trains,
            schedules,
            mut entity_map,
            selection_state,
            type_registry,
            mut commands,
            control_info,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok((mut train, schedule_option)) = trains.get_mut(entity) {
                if ui_for_value(&mut train.settings, ui, &type_registry.read()) {
                    train.update_wagon_entities(&mut commands, &mut entity_map);
                }
                ui.separator();
                ui.heading("Schedule");
                if let Some(mut schedule) = schedule_option {
                    TrainSchedule::selector_option(&schedules, ui, &mut schedule.schedule_id);
                    if let Some(sched) = schedule.schedule_id {
                        let actual_schedule = schedules
                            .get(entity_map.get_entity(&GenericID::Schedule(sched)).unwrap())
                            .unwrap()
                            .0;
                        ui.label(format!(
                            "Cycle time {:1.0}",
                            schedule.cycle_time(control_info.time, &actual_schedule)
                        ));
                    }
                } else {
                    commands.entity(entity).insert(AssignedSchedule::default());
                }
                ui.separator();
            }
        }
        state.apply(world);
    }

    pub fn update_wagon_entities(
        &mut self,
        commands: &mut Commands,
        entity_map: &mut ResMut<EntityMap>,
    ) {
        while self.wagons.len() < self.settings.num_wagons + 1 {
            let wagon_id = WagonID {
                train: self.id,
                index: self.wagons.len(),
            };
            let wagon = TrainWagonBundle::new(wagon_id);
            let entity = commands.spawn(wagon).id();
            entity_map.add_wagon(wagon_id, entity);
            self.wagons.push(wagon_id);
        }
        while self.wagons.len() > self.settings.num_wagons + 1 {
            let wagon_id = self.wagons.pop().unwrap();
            let entity = entity_map.wagons.remove(&wagon_id).unwrap();
            commands.entity(entity).despawn();
        }
    }
}

impl Selectable for Train {
    type SpawnEvent = SpawnTrainEvent;
    type ID = TrainID;

    fn id(&self) -> Self::ID {
        self.id
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Train(self.id)
    }

    fn get_depth(&self) -> f32 {
        3.0
    }
}

#[derive(SystemParam)]
pub struct SpawnTrainEventQuery<'w, 's> {
    trains: Query<
        'w,
        's,
        (
            &'static Train,
            &'static BLETrain,
            &'static Name,
            Option<&'static AssignedSchedule>,
        ),
    >,
}

impl SpawnTrainEventQuery<'_, '_> {
    pub fn get(&self) -> Vec<SpawnTrainEvent> {
        self.trains
            .iter()
            .map(|(train, ble_train, name, schedule)| SpawnTrainEvent {
                train: train.clone(),
                ble_train: Some(ble_train.clone()),
                name: Some(name.to_string()),
                schedule: schedule.cloned(),
            })
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnTrainEvent {
    pub train: Train,
    pub ble_train: Option<BLETrain>,
    pub name: Option<String>,
    pub schedule: Option<AssignedSchedule>,
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

fn update_wagons(
    q_trains: Query<&Train>,
    mut q_wagons: Query<(&mut Transform, &mut Stroke)>,
    entity_map: Res<EntityMap>,
    hover_state: Res<HoverState>,
    selection_state: Res<SelectionState>,
) {
    for train in q_trains.iter() {
        let mut color = Color::from(YELLOW);
        if Selection::Single(GenericID::Train(train.id)) == selection_state.selection {
            color = Color::from(ORANGE);
        }
        if hover_state.hover == Some(GenericID::Train(train.id)) {
            color = Color::from(RED);
        }
        for wagon_id in &train.wagons {
            let wagon_entity = entity_map.wagons.get(wagon_id).unwrap();
            let (mut transform, mut stroke) = q_wagons.get_mut(*wagon_entity).unwrap();
            let offset = -WAGON_DIST * (wagon_id.index as f32);
            let offset2 = offset + train.in_place_cycle * WAGON_DIST;
            let pos = train.get_route().interpolate_offset(offset2);
            let pos2 = train.get_route().interpolate_offset(offset2 + 0.01);
            let angle = -(pos2 - pos).angle_between(Vec2::X);
            transform.translation = pos.extend(20.0) * LAYOUT_SCALE;
            transform.rotation = Quat::from_rotation_z(angle);

            let mut alpha = 1.0;
            if wagon_id.index == 0 {
                alpha = 1.0 - train.in_place_cycle;
            }
            if wagon_id.index == train.settings.num_wagons {
                alpha = train.in_place_cycle;
            }
            stroke.color = color.with_alpha(alpha.powi(1));
        }
    }
}

fn draw_train(mut gizmos: Gizmos, q_trains: Query<&Train>) {
    for train in q_trains.iter() {
        let pos = train.get_route().interpolate_offset(0.0);
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.03 * LAYOUT_SCALE, Color::BLACK);
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
            dirtrack.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::from(RED));
        }
    }
}

fn init_drag_train(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    mut hover_state: ResMut<HoverState>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        if let Some(GenericID::Train(train_id)) = &hover_state.hover {
            train_drag_state.train_id = Some(*train_id);
            train_drag_state.target = None;
            train_drag_state.target_facing = Facing::Forward;
            hover_state.filter = HoverFilter::Blocks;
        }
    }
}

fn exit_drag_train(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    mut set_train_route: EventWriter<SetTrainRouteEvent>,
    mut hover_state: ResMut<HoverState>,
) {
    if mouse_buttons.just_released(MouseButton::Right) {
        if let Some(train_id) = train_drag_state.train_id {
            if let Some(route) = train_drag_state.route.clone() {
                set_train_route.send(SetTrainRouteEvent {
                    train_id,
                    route: route,
                });
            }
        }
        train_drag_state.train_id = None;
        train_drag_state.route = None;
        hover_state.filter = HoverFilter::All;
    }
}

#[derive(Event, Debug)]
pub struct LocksChangedEvent {}

#[derive(Event, Debug)]
pub struct PlanRouteEvent {}

fn assign_destination_route(
    _trigger: Trigger<PlanRouteEvent>,
    q_blocks: Query<&Block>,
    q_destinations: Query<&Destination>,
    entity_map: Res<EntityMap>,
    connections: Res<Connections>,
    track_locks: Res<TrackLocks>,
    q_trains: Query<(&Train, &QueuedDestination)>,
    q_markers: Query<&Marker>,
    switches: Query<&Switch>,
    marker_map: Res<MarkerMap>,
    mut set_train_route: EventWriter<SetTrainRouteEvent>,
) {
    for (train, queue) in q_trains.iter() {
        if !train.get_route().is_blocked() {
            if !train.get_route().is_completed() {
                continue;
            }
            if train.get_route().num_legs() > 1 {
                continue;
            }
        }

        let train_id = train.id;
        let start = train.get_logical_block_id();
        let destination = match queue.dest {
            DestinationID::Specific(_) => q_destinations
                .get(
                    entity_map
                        .get_entity(&GenericID::Destination(queue.dest))
                        .unwrap(),
                )
                .unwrap(),
            DestinationID::Random => &Destination {
                id: DestinationID::Random,
                blocks: q_blocks
                    .iter()
                    .map(|block| (block.id, BlockDirectionFilter::Any, None))
                    .collect(),
            },
        };

        let mut routes = vec![];
        for (block_id, dir, _) in destination.blocks.iter() {
            for direction in dir.iter_directions() {
                let target = block_id.to_logical(*direction, Facing::Forward);
                if target == start {
                    continue;
                }
                if let Some(logical_section) = connections.find_route_section(
                    start,
                    target,
                    Some((&train_id, &track_locks, &switches, &entity_map)),
                    train.settings.prefer_facing,
                ) {
                    let route = build_route(
                        train_id,
                        &logical_section,
                        &q_markers,
                        &q_blocks,
                        &entity_map,
                        &marker_map,
                    );
                    routes.push(route);
                }
            }
        }
        match queue.strategy {
            TargetChoiceStrategy::Closest => {
                routes.sort_by_key(|route| route.total_length());
            }
            TargetChoiceStrategy::Random => {
                routes.shuffle(&mut rand::thread_rng());
            }
        }

        if let Some(route) = routes.first().cloned() {
            set_train_route.send(SetTrainRouteEvent {
                train_id,
                route: route,
            });
        } else {
            println!("No route found for train {:?}", train_id);
        }
        return;
    }
}

fn update_drag_train(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    mouse_pos: Res<MousePosWorld>,
    hover_state: Res<HoverState>,
    q_blocks: Query<&Block>,
    entity_map: Res<EntityMap>,
    connections: Res<Connections>,
    track_locks: Res<TrackLocks>,
    q_trains: Query<&Train>,
    q_markers: Query<&Marker>,
    switches: Query<&Switch>,
    marker_map: Res<MarkerMap>,
) {
    if train_drag_state.train_id.is_none() {
        return;
    }
    let old_target = train_drag_state.target.clone();
    if mouse_buttons.just_pressed(MouseButton::Left) {
        train_drag_state.target_facing = train_drag_state.target_facing.opposite();
        println!("Target facing: {:?}", train_drag_state.target_facing)
    }
    if let Some(GenericID::Block(block_id)) = hover_state.hover {
        let block = q_blocks
            .get(entity_map.get_entity(&GenericID::Block(block_id)).unwrap())
            .unwrap();
        train_drag_state.target = Some(block_id.to_logical(
            block.hover_pos_to_direction(mouse_pos.truncate() / LAYOUT_SCALE),
            train_drag_state.target_facing,
        ));
        if train_drag_state.target == old_target {
            return;
        }

        // build route
        let train_id = train_drag_state.train_id.unwrap();
        let train = q_trains
            .get(entity_map.get_entity(&GenericID::Train(train_id)).unwrap())
            .unwrap();
        let start = train.get_logical_block_id();
        if let Some(logical_section) = connections.find_route_section(
            start,
            train_drag_state.target.unwrap(),
            Some((&train_id, &track_locks, &switches, &entity_map)),
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
            train_drag_state.route = Some(route);
        } else {
            train_drag_state.route = None;
        }
    } else {
        train_drag_state.target = None;
        train_drag_state.route = None;
    }
}

fn draw_hover_route(mut gizmos: Gizmos, train_drag_state: Res<TrainDragState>) {
    if let Some(route) = train_drag_state.route.clone() {
        route.draw_with_gizmos(&mut gizmos);
    }
}

#[derive(Event)]
pub struct MarkerAdvanceEvent {
    pub id: TrainID,
    pub index: usize,
}

#[derive(Debug, Component)]
pub struct WaitTime {
    pub time: f32,
}

impl WaitTime {
    pub fn new() -> WaitTime {
        WaitTime { time: 0.0 }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TargetChoiceStrategy {
    Random,
    Closest,
}

#[derive(Debug, Component)]
pub struct QueuedDestination {
    pub dest: DestinationID,
    pub strategy: TargetChoiceStrategy,
    pub allow_locked: bool,
}

#[derive(Debug, Event)]
pub struct SetTrainRouteEvent {
    train_id: TrainID,
    route: Route,
}

fn tick_wait_time(mut q_times: Query<&mut WaitTime>, time: Res<Time>) {
    for mut wait_time in q_times.iter_mut() {
        wait_time.time += time.delta_seconds();
        if (wait_time.time - time.delta_seconds()) % 1.0 > wait_time.time % 1.0 {
            println!("Wait time: {:1.0}s", wait_time.time);
        }
    }
}

pub fn set_train_route(
    mut q_trains: Query<(&mut Train, &mut BLETrain)>,
    switches: Query<&Switch>,
    entity_map: Res<EntityMap>,
    mut route_events: EventReader<SetTrainRouteEvent>,
    mut track_locks: ResMut<TrackLocks>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
    editor_state: Res<State<EditorState>>,
    mut hub_commands: EventWriter<HubCommandEvent>,
    mut commands: Commands,
) {
    for event in route_events.read() {
        let mut route = event.route.clone();

        let train_entity = entity_map
            .get_entity(&GenericID::Train(event.train_id))
            .unwrap();
        if route.num_legs() > 1 {
            commands.entity(train_entity).remove::<WaitTime>();
        } else {
            commands.entity(train_entity).remove::<QueuedDestination>();
        }
        let (mut train, ble_train) = q_trains.get_mut(train_entity).unwrap();
        // println!("Dropping train {:?} on block {:?}", train_id, block_id);
        route.pretty_print();
        route.get_current_leg_mut().set_signed_pos_from_last(
            train
                .get_route()
                .get_current_leg()
                .get_signed_pos_from_last(),
        );

        // route.get_current_leg_mut().intention = LegIntention::Stop;
        train.position = Position::Route(route);

        if update_train_route(
            &mut train,
            &mut track_locks,
            &switches,
            &entity_map,
            &mut set_switch_position,
        ) {
            commands.trigger(LocksChangedEvent {});
        }
        train.set_seek_target();

        if editor_state.get().ble_commands_enabled() {
            let commands = ble_train.download_route(&train.get_route());
            for input in commands.hub_events {
                info!("Sending {:?}", input);
                hub_commands.send(input);
            }
        }
    }
}

fn create_train_shortcut(
    keyboard_input: Res<ButtonInput<keyboard::KeyCode>>,
    mut train_events: EventWriter<SpawnTrainEvent>,
    entity_map: Res<EntityMap>,
    selection_state: Res<SelectionState>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::KeyT) {
        if let Selection::Single(GenericID::Block(block_id)) = &selection_state.selection {
            // println!("Creating train at block {:?}", block_id);
            let logical_block_id = block_id.to_logical(BlockDirection::Aligned, Facing::Forward);
            let train_id = entity_map.new_train_id();
            let train = Train::at_block_id(train_id, logical_block_id);
            train_events.send(SpawnTrainEvent {
                train,
                ble_train: None,
                name: None,
                schedule: None,
            });
        }
    }
}

fn spawn_train(
    mut train_events: EventReader<SpawnTrainEvent>,
    mut commands: Commands,
    q_blocks: Query<&Block>,
    mut track_locks: ResMut<TrackLocks>,
    mut entity_map: ResMut<EntityMap>,
    marker_map: Res<MarkerMap>,
    q_markers: Query<&Marker>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
    switches: Query<&Switch>,
) {
    for spawn_train in train_events.read() {
        let serialized_train = spawn_train.clone();
        let mut train = serialized_train.train;
        let block_id = match train.position {
            Position::Storage => {
                panic!("Can't spawn train in storage")
            }
            Position::Block(block_id) => block_id,
            Position::Route(_) => panic!("Can't spawn train with route"),
        };
        println!("spawning at block {:?}", block_id);
        let train_id = TrainID::new(entity_map.trains.len());
        let route = block_route(
            block_id,
            train_id,
            &q_markers,
            &q_blocks,
            &entity_map,
            &marker_map,
        );
        train.position = Position::Route(route);
        if update_train_route(
            &mut train,
            &mut track_locks,
            &switches,
            &entity_map,
            &mut set_switch_position,
        ) {
            commands.trigger(LocksChangedEvent {});
        }
        println!("train block: {:?}", train.get_logical_block_id());
        let mut train = TrainBundle::from_train(train);
        train
            .train
            .update_wagon_entities(&mut commands, &mut entity_map);
        let train_id = train.train.id;
        // println!("Section: {:?}", block_section);
        // println!("Layout markers: {:?}", entity_map.markers);
        /*println!(
            "Creating train {:?} at logical block {:?}",
            train_id, logical_block_id
        );*/
        let ble_train = serialized_train
            .ble_train
            .unwrap_or(BLETrain::new(train_id));
        let name = Name::new(spawn_train.name.clone().unwrap_or(train_id.to_string()));
        let schedule = spawn_train
            .schedule
            .clone()
            .unwrap_or(AssignedSchedule::default());
        let entity = commands
            .spawn((name, train, ble_train, WaitTime::new(), schedule))
            .id();
        entity_map.add_train(train_id, entity);
    }
}

fn block_route(
    block_id: LogicalBlockID,
    train_id: TrainID,
    q_markers: &Query<&Marker>,
    q_blocks: &Query<&Block>,
    entity_map: &EntityMap,
    marker_map: &MarkerMap,
) -> Route {
    let mut section = LogicalSection::new();
    section.tracks.push(block_id.default_in_marker_track());
    let route = build_route(
        train_id, &section, q_markers, q_blocks, entity_map, marker_map,
    );
    route
}

fn despawn_train(
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut despawn_events: EventReader<DespawnEvent<Train>>,
    mut track_locks: ResMut<TrackLocks>,
) {
    for event in despawn_events.read() {
        let train_id = event.0.id;
        let entity = entity_map.trains.get(&train_id).unwrap();
        track_locks.unlock_all(&train_id);
        commands.entity(*entity).despawn();
        entity_map.remove_train(train_id);
    }
}

fn update_virtual_trains(
    mut q_trains: Query<&mut Train>,
    time: Res<Time>,
    mut advance_events: EventWriter<MarkerAdvanceEvent>,
) {
    for mut train in q_trains.iter_mut() {
        train.traverse_route(time.delta_seconds(), &mut advance_events);
    }
}

fn update_train_route(
    train: &mut Train,
    track_locks: &mut TrackLocks,
    switches: &Query<&Switch>,
    entity_map: &EntityMap,
    set_switch_position: &mut EventWriter<SetSwitchPositionEvent>,
) -> bool {
    train
        .get_route_mut()
        .update_intentions(track_locks, switches, entity_map);
    let old_locks = track_locks.clone();
    train
        .get_route()
        .update_locks(track_locks, entity_map, set_switch_position, switches);
    *track_locks != old_locks
}

fn update_virtual_trains_passive(mut q_trains: Query<&mut Train>, time: Res<Time>) {
    for mut train in q_trains.iter_mut() {
        train.traverse_route_passive(time.delta_seconds());
    }
}

fn trigger_manual_sensor_advance(
    mut events: EventWriter<MarkerAdvanceEvent>,
    keyboard_input: Res<ButtonInput<keyboard::KeyCode>>,
    selection_state: Res<SelectionState>,
    mut trains: Query<&mut Train>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::KeyN) {
        if let Selection::Single(GenericID::Train(train_id)) = selection_state.selection {
            let mut train = trains
                .get_mut(entity_map.get_entity(&GenericID::Train(train_id)).unwrap())
                .unwrap();
            let route = train.get_route_mut();
            if route.get_current_leg().get_leg_state() != LegState::Completed {
                println!("Advancing marker");
                events.send(MarkerAdvanceEvent {
                    id: train_id,
                    index: route.get_current_leg().index + 1,
                });
            }
        }
    }
}

fn sensor_advance(
    mut q_trains: Query<&mut Train, With<BLETrain>>,
    q_markers: Query<&Marker>,
    q_blocks: Query<&Block>,
    marker_map: Res<MarkerMap>,
    mut ble_sensor_advance_events: EventReader<MarkerAdvanceEvent>,
    entity_map: Res<EntityMap>,
    mut track_locks: ResMut<TrackLocks>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
    mut commands: Commands,
    switches: Query<&Switch>,
    mut set_train_route: EventWriter<SetTrainRouteEvent>,
) {
    for advance in ble_sensor_advance_events.read() {
        info!("Advancing sensor for train {:?}", advance.id);
        let train_entity = entity_map
            .get_entity(&GenericID::Train(advance.id))
            .unwrap();
        let mut train = q_trains.get_mut(train_entity).unwrap();
        assert_eq!(advance.index, train.get_route().get_current_leg().index + 1);
        train.advance_sensor();
        if update_train_route(
            &mut train,
            &mut track_locks,
            &switches,
            &entity_map,
            &mut set_switch_position,
        ) {
            commands.trigger(LocksChangedEvent {});
        }

        if train.get_route().is_completed() {
            println!("Train {:?} completed route", train.id);
            commands.entity(train_entity).insert(WaitTime::new());
            let route = block_route(
                train.get_route().get_current_leg().get_target_block_id(),
                train.id,
                &q_markers,
                &q_blocks,
                &entity_map,
                &marker_map,
            );
            set_train_route.send(SetTrainRouteEvent {
                train_id: train.id,
                route: route,
            });
        }
    }
}

fn update_routes(
    _trigger: Trigger<LocksChangedEvent>,
    mut q_trains: Query<&mut Train>,
    mut track_locks: ResMut<TrackLocks>,
    switches: Query<&Switch>,
    entity_map: Res<EntityMap>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
    mut commands: Commands,
) {
    for mut train in q_trains.iter_mut() {
        if update_train_route(
            &mut train,
            &mut track_locks,
            &switches,
            &entity_map,
            &mut set_switch_position,
        ) {
            commands.trigger(LocksChangedEvent {});
            return;
        }
    }
    // only triggered if no locks changed this run
    commands.trigger(PlanRouteEvent {});
}

fn sync_intentions(
    mut q_trains: Query<(&mut Train, &BLETrain)>,
    mut hub_commands: EventWriter<HubCommandEvent>,
) {
    for (mut train, ble_train) in q_trains.iter_mut() {
        let route = train.get_route_mut();
        for (leg_index, leg) in route.iter_legs_mut().enumerate() {
            if leg.intention_synced {
                continue;
            }
            let commands = ble_train.set_leg_intention(leg_index as u8, leg.intention);
            println!(
                "Setting intention for leg {}: {:?}",
                leg_index, leg.intention
            );
            leg.intention_synced = true;
            for input in commands.hub_events {
                hub_commands.send(input);
            }
        }
    }
}

pub struct TrainPlugin;

impl Plugin for TrainPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Facing>();
        app.insert_resource(TrainDragState::default());
        app.add_event::<SetTrainRouteEvent>();
        app.add_event::<DespawnEvent<Train>>();
        app.observe(assign_destination_route);
        app.observe(update_routes);
        app.add_systems(
            Update,
            (
                create_train_shortcut,
                delete_selection_shortcut::<Train>,
                despawn_train.run_if(on_event::<DespawnEvent<Train>>()),
                draw_train,
                update_wagons.after(directory_panel),
                // draw_train_route.after(draw_hover_route),
                // draw_locked_tracks.after(draw_train_route),
                // draw_hover_route,
                init_drag_train.after(finish_hover),
                exit_drag_train,
                tick_wait_time.run_if(in_state(ControlState)),
                set_train_route.run_if(on_event::<SetTrainRouteEvent>()),
                update_drag_train.after(finish_hover),
                update_virtual_trains
                    .run_if(in_state(EditorState::VirtualControl))
                    .after(sensor_advance),
                update_virtual_trains_passive
                    .run_if(in_state(EditorState::DeviceControl))
                    .after(sensor_advance),
                sensor_advance.run_if(on_event::<MarkerAdvanceEvent>()),
                sync_intentions
                    .run_if(in_state(EditorState::DeviceControl))
                    .after(update_virtual_trains_passive),
                trigger_manual_sensor_advance.run_if(in_state(EditorState::DeviceControl)),
            ),
        );
        app.add_systems(
            PreUpdate,
            spawn_train
                .run_if(on_event::<SpawnTrainEvent>())
                .after(spawn_block),
        );
    }
}
