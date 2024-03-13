use crate::{
    ble::HubCommandEvent,
    ble_train::{BLESensorAdvanceEvent, BLETrain},
    block::{spawn_block, Block},
    editor::*,
    layout::{Connections, EntityMap, MarkerMap, TrackLocks},
    layout_primitives::*,
    marker::Marker,
    route::{build_route, Route, TrainState},
    switch::SetSwitchPositionEvent,
    track::LAYOUT_SCALE,
    train,
};
use bevy::{input::keyboard, prelude::*};
use bevy_ecs::system::SystemState;
use bevy_egui::egui::Ui;
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use bevy_mouse_tracking_plugin::MousePosWorld;
use bevy_prototype_lyon::{
    draw::Stroke,
    entity::ShapeBundle,
    path::ShapePath,
    prelude::{LineCap, StrokeOptions},
    shapes::Line,
};
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

const TRAIN_WIDTH: f32 = 0.1;

#[derive(Resource, Default, Debug)]
struct TrainDragState {
    train_id: Option<TrainID>,
    target: Option<LogicalBlockID>,
    target_facing: Facing,
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
    id: TrainID,
    position: Position,
    #[serde(skip)]
    state: TrainState,
    #[serde(skip)]
    speed: f32,
    settings: TrainSettings,
}

impl Train {
    pub fn at_block_id(train_id: TrainID, logical_block_id: LogicalBlockID) -> Train {
        let train = Train {
            id: train_id,
            position: Position::Block(logical_block_id),
            state: TrainState::Stop,
            speed: 0.0,
            settings: TrainSettings {
                num_wagons: 3,
                home: None,
                prefer_facing: None,
            },
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

    fn traverse_route(&mut self, delta: f32) -> bool {
        let dist = delta * self.speed;
        let change_locks = self.get_route_mut().advance_distance(dist);
        self.state = self.get_route().get_train_state();
        self.speed = self.state.get_speed();
        return change_locks;
        // println!("Train state: {:?}, {:?}", self.state, self.speed);
        // println!("Route: {:?}", self.route.get_current_leg().section_position);
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Train>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
        )>::new(world);
        let (mut trains, entity_map, selection_state, type_registry) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut train) = trains.get_mut(entity) {
                ui_for_value(&mut train.settings, ui, &type_registry.read());
                ui.separator();
            }
        }
    }
}

impl Selectable for Train {
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    hover_state: Res<HoverState>,
) {
    if mouse_buttons.just_pressed(MouseButton::Right) {
        if let Some(GenericID::Train(train_id)) = hover_state.hover {
            train_drag_state.train_id = Some(train_id);
            train_drag_state.target = None;
            train_drag_state.target_facing = Facing::Forward;
            println!("Dragging train {:?}", train_id)
        }
    }
}

fn exit_drag_train(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut train_drag_state: ResMut<TrainDragState>,
    entity_map: Res<EntityMap>,
    marker_map: Res<MarkerMap>,
    connections: Res<Connections>,
    mut track_locks: ResMut<TrackLocks>,
    mut q_trains: Query<(&mut Train, &mut BLETrain)>,
    mut hub_commands: EventWriter<HubCommandEvent>,
    q_blocks: Query<&Block>,
    q_markers: Query<&Marker>,
    editor_state: Res<State<EditorState>>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
) {
    if mouse_buttons.just_released(MouseButton::Right) {
        if let Some(train_id) = train_drag_state.train_id {
            if let Some(target) = train_drag_state.target {
                let (mut train, ble_train) = q_trains
                    .get_mut(entity_map.get_entity(&GenericID::Train(train_id)).unwrap())
                    .unwrap();
                // println!("Dropping train {:?} on block {:?}", train_id, block_id);
                let start = train.get_logical_block_id();
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
                    route.pretty_print();
                    // route.get_current_leg_mut().intention = LegIntention::Stop;
                    train.position = Position::Route(route);

                    train
                        .get_route_mut()
                        .update_intentions(track_locks.as_ref());
                    // println!("state: {:?}", train.route.get_train_state());
                    train.get_route().update_locks(
                        &mut track_locks,
                        &entity_map,
                        &mut set_switch_position,
                    );

                    if editor_state.get().ble_commands_enabled() {
                        let commands = ble_train.download_route(&train.get_route());
                        for input in commands.hub_events {
                            info!("Sending {:?}", input);
                            hub_commands.send(input);
                        }
                    }
                }
            }
            train_drag_state.train_id = None;
        }
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
    marker_map: Res<MarkerMap>,
    mut gizmos: Gizmos,
) {
    if train_drag_state.train_id.is_none() {
        return;
    }
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
        let train_id = train_drag_state.train_id.unwrap();
        let train = q_trains
            .get(entity_map.get_entity(&GenericID::Train(train_id)).unwrap())
            .unwrap();
        let start = train.get_logical_block_id();
        if let Some(logical_section) = connections.find_route_section(
            start,
            train_drag_state.target.unwrap(),
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
            route.draw_with_gizmos(&mut gizmos);
        }
    } else {
        train_drag_state.target = None;
    }
}

fn create_train(
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
        route.update_locks(&mut track_locks, &entity_map, &mut set_switch_position);
        train.position = Position::Route(route);
        println!("train block: {:?}", train.get_logical_block_id());
        let train = TrainBundle::from_train(train);
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
        let entity = commands.spawn((train, ble_train)).id();
        entity_map.add_train(train_id, entity);
    }
}

fn update_virtual_trains(
    mut q_trains: Query<&mut Train>,
    time: Res<Time>,
    mut track_locks: ResMut<TrackLocks>,
    entity_map: Res<EntityMap>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
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
            train
                .get_route()
                .update_locks(&mut track_locks, &entity_map, &mut set_switch_position);
        }
    }
}

fn handle_ble_sensor_advance(
    mut q_trains: Query<&mut Train, With<BLETrain>>,
    mut ble_sensor_advance_events: EventReader<BLESensorAdvanceEvent>,
    entity_map: Res<EntityMap>,
    mut track_locks: ResMut<TrackLocks>,
    mut set_switch_position: EventWriter<SetSwitchPositionEvent>,
) {
    for advance in ble_sensor_advance_events.read() {
        let mut train = q_trains
            .get_mut(
                entity_map
                    .get_entity(&GenericID::Train(advance.id))
                    .unwrap(),
            )
            .unwrap();
        let route = train.get_route_mut();
        route.advance_sensor();
        route.update_locks(&mut track_locks, &entity_map, &mut set_switch_position);

        for mut train in q_trains.iter_mut() {
            if !track_locks.is_clean(&train.id) {
                train
                    .get_route_mut()
                    .update_intentions(track_locks.as_ref());
                track_locks.mark_clean(&train.id);
            }
        }
    }
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
        app.register_component_as::<dyn Selectable, Train>();
        app.register_type::<Facing>();
        app.insert_resource(TrainDragState::default());
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
                update_virtual_trains.run_if(in_state(EditorState::VirtualControl)),
                handle_ble_sensor_advance.run_if(on_event::<BLESensorAdvanceEvent>()),
                sync_intentions.run_if(in_state(EditorState::DeviceControl)),
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
