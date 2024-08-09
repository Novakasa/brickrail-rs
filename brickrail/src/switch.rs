use bevy::color::palettes::css::{BLUE, GRAY, MAGENTA};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::{color::palettes::css::RED, ecs::system::SystemState};
use bevy_egui::egui::Ui;
use bevy_inspector_egui::bevy_egui;
use bevy_prototype_lyon::draw::Stroke;
use bevy_prototype_lyon::entity::ShapeBundle;
use bevy_prototype_lyon::prelude::{LineCap, StrokeOptions};
use serde::{Deserialize, Serialize};

use crate::editor::{directory_panel, HoverState, Selection};
use crate::track::build_connection_path_extents;
use crate::{
    ble::{BLEHub, HubCommandEvent},
    editor::{DespawnEvent, EditorState, GenericID, Selectable, SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_devices::{select_device_id, LayoutDevice},
    layout_primitives::*,
    switch_motor::{MotorPosition, SpawnSwitchMotorEvent, SwitchMotor},
    track::{spawn_connection, LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Switch {
    id: DirectedTrackID,
    positions: Vec<SwitchPosition>,
    pub motors: Vec<Option<LayoutDeviceID>>,
}

impl Switch {
    pub fn new(id: DirectedTrackID, positions: Vec<SwitchPosition>) -> Self {
        let mut switch = Self {
            id,
            positions: Vec::new(),
            motors: Vec::new(),
        };
        switch.set_positions(positions);
        switch
    }

    pub fn set_positions(&mut self, positions: Vec<SwitchPosition>) {
        self.motors
            .resize_with(positions.len() - 1, Default::default);

        self.positions = positions;
        self.positions.sort();
    }

    pub fn get_position(
        &self,
        motor_positions: &Vec<Option<MotorPosition>>,
    ) -> Option<SwitchPosition> {
        if motor_positions.len() == 2 {
            match (motor_positions[0].clone(), motor_positions[1].clone()) {
                (Some(MotorPosition::Left), Some(MotorPosition::Left)) => {
                    return Some(SwitchPosition::Left);
                }
                (Some(MotorPosition::Left), Some(MotorPosition::Right)) => {
                    return None;
                }
                (Some(MotorPosition::Right), Some(MotorPosition::Right)) => {
                    return Some(SwitchPosition::Right);
                }
                (Some(MotorPosition::Right), Some(MotorPosition::Left)) => {
                    return Some(SwitchPosition::Center);
                }
                _ => {
                    return None;
                }
            }
        }
        if motor_positions.len() == 1 {
            match motor_positions[0] {
                Some(MotorPosition::Left) => {
                    return Some(self.positions[0].clone());
                }
                Some(MotorPosition::Right) => {
                    return Some(self.positions[1].clone());
                }
                _ => {
                    return None;
                }
            }
        }
        panic!("Invalid motor positions");
    }

    pub fn iter_motor_positions(
        &self,
        pos: &SwitchPosition,
    ) -> impl Iterator<Item = (&Option<LayoutDeviceID>, MotorPosition)> {
        let pos_index = self.positions.iter().position(|p| p == pos).unwrap();
        self.motors
            .iter()
            .enumerate()
            .map(move |(index, motor_id)| {
                let position = match (pos_index, index) {
                    (0, 0) => MotorPosition::Left,
                    (0, 1) => MotorPosition::Left,
                    (1, 0) => MotorPosition::Right,
                    (1, 1) => MotorPosition::Left,
                    (2, 0) => MotorPosition::Right,
                    (2, 1) => MotorPosition::Right,
                    _ => panic!("Invalid switch position"),
                };
                (motor_id, position)
            })
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut Switch>,
            ResMut<EntityMap>,
            ResMut<SelectionState>,
            Res<AppTypeRegistry>,
            Query<&BLEHub>,
            EventWriter<SpawnHubEvent>,
            EventWriter<SpawnSwitchMotorEvent>,
            EventWriter<DespawnEvent<LayoutDevice>>,
            Query<(&mut SwitchMotor, &mut LayoutDevice)>,
            EventWriter<SetSwitchPositionEvent>,
        )>::new(world);
        let (
            mut switches,
            mut entity_map,
            mut selection_state,
            type_registry,
            hubs,
            mut spawn_events,
            mut spawn_devices,
            mut despawn_devices,
            mut devices,
            mut set_switch_position,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut switch) = switches.get_mut(entity) {
                ui.heading("Switch");
                ui.label("position");
                ui.horizontal(|ui| {
                    for position in switch.positions.clone() {
                        if ui.button(position.to_string()).clicked() {
                            set_switch_position.send(SetSwitchPositionEvent {
                                id: switch.id,
                                position,
                            });
                        }
                    }
                });
                ui.separator();
                for (i, motor_id) in &mut switch.motors.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.heading(format!("Motor {:}", i));
                        select_device_id(
                            ui,
                            motor_id,
                            &mut devices,
                            &mut spawn_devices,
                            &mut despawn_devices,
                            &mut entity_map,
                            &hubs,
                        );
                        if let Some(motor_id) = motor_id {
                            if let Some(entity) = entity_map.layout_devices.get(motor_id) {
                                if let Ok((mut motor, mut device)) = devices.get_mut(*entity) {
                                    device.inspector(
                                        ui,
                                        &hubs,
                                        &mut spawn_events,
                                        &mut entity_map,
                                        &mut selection_state,
                                    );
                                    motor.inspector(ui, &type_registry.read());
                                }
                            }
                        }
                    });
                    ui.separator();
                }
            }
        }
    }
}

impl Selectable for Switch {
    type SpawnEvent = SpawnSwitchEvent;
    type ID = DirectedTrackID;

    fn generic_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }

    fn id(&self) -> Self::ID {
        self.id
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Stroke>,
    ) -> f32 {
        self.id
            .to_slot()
            .get_vec2()
            .lerp(self.id.from_slot().get_vec2(), 0.1)
            .distance(pos)
            - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnSwitchEvent {
    pub switch: Switch,
    pub name: Option<String>,
}

#[derive(SystemParam)]
pub struct SpawnSwitchEventQuery<'w, 's> {
    query: Query<'w, 's, (&'static Switch, &'static Name)>,
}
impl SpawnSwitchEventQuery<'_, '_> {
    pub fn get(&self) -> Vec<SpawnSwitchEvent> {
        self.query
            .iter()
            .map(|(switch, name)| SpawnSwitchEvent {
                switch: switch.clone(),
                name: Some(name.to_string()),
            })
            .collect()
    }
}

#[derive(Debug, Event)]
pub struct UpdateSwitchTurnsEvent {
    pub id: DirectedTrackID,
    pub positions: Vec<SwitchPosition>,
}

#[derive(Debug, Event)]
pub struct SetSwitchPositionEvent {
    pub id: DirectedTrackID,
    pub position: SwitchPosition,
}

pub fn update_switch_position(
    mut events: EventReader<SetSwitchPositionEvent>,
    switches: Query<&Switch>,
    mut switch_motors: Query<(&mut SwitchMotor, &LayoutDevice)>,
    entity_map: Res<EntityMap>,
    mut hub_commands: EventWriter<HubCommandEvent>,
    editor_state: Res<State<EditorState>>,
) {
    for update in events.read() {
        if let Some(entity) = entity_map.switches.get(&update.id) {
            let switch = switches.get(*entity).unwrap();
            for (motor_id, position) in switch.iter_motor_positions(&update.position) {
                if let Some(motor_id) = motor_id {
                    let entity = entity_map.layout_devices.get(motor_id).unwrap();
                    let (mut motor, device) = switch_motors.get_mut(*entity).unwrap();
                    if motor.position == position {
                        continue;
                    }

                    if editor_state.get().ble_commands_enabled() {
                        if let Some(command) = SwitchMotor::switch_command(device, &position) {
                            println!("Sending switch command {:?}", command);
                            hub_commands.send(command);
                        }
                    }
                    motor.position = position;
                }
            }
        }
    }
}

pub fn update_switch_turns(
    mut events: EventReader<UpdateSwitchTurnsEvent>,
    mut switch_spawn_events: EventWriter<SpawnSwitchEvent>,
    mut despawn_switch_events: EventWriter<DespawnEvent<Switch>>,
    mut switches: Query<&mut Switch>,
    entity_map: Res<EntityMap>,
    mut commands: Commands,
    switch_connections: Query<(Entity, &SwitchConnection)>,
) {
    for update in events.read() {
        if update.positions.len() > 1 {
            if let Some(entity) = entity_map.switches.get(&update.id) {
                let mut switch = switches.get_mut(*entity).unwrap();
                switch.set_positions(update.positions.clone());
            } else {
                switch_spawn_events.send(SpawnSwitchEvent {
                    switch: Switch::new(update.id, update.positions.clone()),
                    name: None,
                });
            }
        } else {
            if let Some(entity) = entity_map.switches.get(&update.id) {
                let switch = switches.get(entity.clone()).unwrap();
                despawn_switch_events.send(DespawnEvent(switch.clone()));
            }
        }
        for (entity, connection) in switch_connections.iter() {
            if update.id == connection.connection.from_track {
                if !update
                    .positions
                    .contains(&connection.connection.to_track.get_switch_position())
                {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
        let mut matched_positions = update.positions.clone();
        for (_, connection) in switch_connections.iter() {
            if update.id == connection.connection.from_track {
                matched_positions
                    .retain(|pos| pos != &connection.connection.to_track.get_switch_position());
            }
        }
        if let Some(switch_entity) = entity_map.switches.get(&update.id) {
            for pos in matched_positions {
                let connection = update.id.get_switch_connection(&pos);
                commands.entity(*switch_entity).with_children(|builder| {
                    builder.spawn(SwitchConnectionBundle::new(connection));
                });
            }
        }
    }
}

pub fn draw_switches(mut gizmos: Gizmos, switches: Query<&Switch>) {
    for switch in switches.iter() {
        let pos = switch
            .id
            .to_slot()
            .get_vec2()
            .lerp(switch.id.from_slot().get_vec2(), 0.1);
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.1 * LAYOUT_SCALE, Color::from(RED));
    }
}

pub fn spawn_switch(
    mut commands: Commands,
    mut events: EventReader<SpawnSwitchEvent>,
    mut entity_map: ResMut<EntityMap>,
) {
    for spawn_event in events.read() {
        let switch = spawn_event.switch.clone();
        let name = Name::new(
            spawn_event
                .name
                .clone()
                .unwrap_or(spawn_event.switch.id.to_string()),
        );
        let entity = commands
            .spawn((name, spawn_event.switch.clone(), SpatialBundle::default()))
            .with_children(|builder| {
                for connection in switch
                    .positions
                    .iter()
                    .map(|pos| switch.id.get_switch_connection(pos))
                {
                    builder.spawn(SwitchConnectionBundle::new(connection));
                }
            })
            .id();
        entity_map.add_switch(spawn_event.switch.id, entity);
    }
}

#[derive(Component, Debug)]
pub struct SwitchConnection {
    pub connection: DirectedTrackConnectionID,
}

#[derive(Bundle)]
pub struct SwitchConnectionBundle {
    connection: SwitchConnection,
}

impl SwitchConnectionBundle {
    pub fn new(connection: DirectedTrackConnectionID) -> Self {
        let straight_length = connection.from_track.straight_length();
        Self {
            connection: SwitchConnection { connection },
        }
    }
}

fn update_switch_shapes(
    switches: Query<&Switch>,
    switch_motors: Query<&SwitchMotor>,
    mut connections: Query<(&SwitchConnection, &mut Stroke, &mut Transform)>,
    hover_state: Res<HoverState>,
    selection_state: Res<SelectionState>,
    entity_map: Res<EntityMap>,
) {
    for (connection, mut stroke, mut transform) in connections.iter_mut() {
        let switch = switches
            .get(entity_map.switches[&connection.connection.from_track])
            .unwrap();
        let positions = switch
            .motors
            .iter()
            .map(|motor_id| {
                motor_id
                    .and_then(|id| entity_map.layout_devices.get(&id))
                    .and_then(|entity| switch_motors.get(*entity).ok())
                    .map(|motor| motor.position.clone())
            })
            .collect::<Vec<Option<MotorPosition>>>();
        let position = switch.get_position(&positions);
        let mut color;
        if position == Some(connection.connection.get_switch_position()) {
            color = Color::from(MAGENTA);
            transform.translation.z = 35.0;
        } else {
            color = Color::from(GRAY);
            transform.translation.z = 30.0;
        }

        if selection_state.selection
            == Selection::Single(GenericID::Switch(connection.connection.from_track))
        {
            color = Color::from(BLUE);
            transform.translation.z = 36.0;
        }
        if hover_state.hover == Some(GenericID::Switch(connection.connection.from_track)) {
            color = Color::from(RED);
            transform.translation.z = 40.0;
        }
        stroke.color = color;
    }
}

pub fn despawn_switch(
    mut commands: Commands,
    mut events: EventReader<DespawnEvent<Switch>>,
    mut entity_map: ResMut<EntityMap>,
) {
    for despawn_event in events.read() {
        if let Some(entity) = entity_map.switches.get(&despawn_event.0.id) {
            commands.entity(*entity).despawn_recursive();
            entity_map.remove_switch(despawn_event.0.id);
        }
    }
}

pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSwitchEvent>();
        app.add_event::<UpdateSwitchTurnsEvent>();
        app.add_event::<SetSwitchPositionEvent>();
        app.add_event::<DespawnEvent<Switch>>();
        app.add_systems(
            Update,
            (
                spawn_switch.run_if(on_event::<SpawnSwitchEvent>()),
                update_switch_shapes.after(directory_panel),
                update_switch_turns
                    .after(spawn_connection)
                    .run_if(on_event::<UpdateSwitchTurnsEvent>()),
                update_switch_position.run_if(on_event::<SetSwitchPositionEvent>()),
                // draw_switches,
                despawn_switch.run_if(on_event::<DespawnEvent<Switch>>()),
            ),
        );
    }
}
