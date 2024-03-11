use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::egui::Ui;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use crate::{
    ble::BLEHub,
    editor::{GenericID, Selectable, SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_devices::{select_device_id, LayoutDevice},
    layout_primitives::*,
    switch_motor::{SpawnSwitchMotorEvent, SwitchMotor},
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Switch {
    id: DirectedTrackID,
    positions: Vec<SwitchPosition>,
    #[serde(skip)]
    pos_index: usize,
    motors: Vec<Option<LayoutDeviceID>>,
}

impl Switch {
    pub fn new(id: DirectedTrackID, positions: Vec<SwitchPosition>) -> Self {
        let mut switch = Self {
            id,
            positions: Vec::new(),
            pos_index: 0,
            motors: Vec::new(),
        };
        switch.set_positions(positions);
        switch
    }

    pub fn set_positions(&mut self, positions: Vec<SwitchPosition>) {
        self.pos_index = 0;
        self.motors
            .resize_with(positions.len() - 1, Default::default);

        self.positions = positions;
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
            Query<(&mut SwitchMotor, &mut LayoutDevice)>,
        )>::new(world);
        let (
            mut switches,
            mut entity_map,
            mut selection_state,
            type_registry,
            hubs,
            mut spawn_events,
            mut spawn_devices,
            mut devices,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut switch) = switches.get_mut(entity) {
                ui.label("BLE Switch");
                for (i, motor_id) in &mut switch.motors.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.label(format!("Motor {:}", i));
                        select_device_id(
                            ui,
                            motor_id,
                            &mut devices,
                            &mut spawn_devices,
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
    fn get_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.id.to_slot().get_vec2().distance(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnSwitchEvent {
    pub switch: Switch,
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
    mut switches: Query<&mut Switch>,
    entity_map: Res<EntityMap>,
) {
    for update in events.read() {
        if let Some(entity) = entity_map.switches.get(&update.id) {
            let mut switch = switches.get_mut(*entity).unwrap();
        }
    }
}

pub fn update_switch_turns(
    mut events: EventReader<UpdateSwitchTurnsEvent>,
    mut switch_spawn_events: EventWriter<SpawnSwitchEvent>,
    mut switches: Query<&mut Switch>,
    entity_map: Res<EntityMap>,
) {
    for update in events.read() {
        if update.positions.len() > 1 {
            if let Some(entity) = entity_map.switches.get(&update.id) {
                let mut switch = switches.get_mut(*entity).unwrap();
                switch.set_positions(update.positions.clone());
            } else {
                switch_spawn_events.send(SpawnSwitchEvent {
                    switch: Switch::new(update.id, update.positions.clone()),
                });
            }
        } else {
            //todo!("Remove switches")
        }
    }
}

pub fn draw_switches(mut gizmos: Gizmos, switches: Query<&Switch>) {
    for switch in switches.iter() {
        let pos = switch.id.to_slot().get_vec2();
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.1 * LAYOUT_SCALE, Color::RED);
    }
}

pub fn spawn_switch(
    mut commands: Commands,
    mut events: EventReader<SpawnSwitchEvent>,
    mut entity_map: ResMut<EntityMap>,
) {
    for spawn_event in events.read() {
        let entity = commands.spawn(spawn_event.switch.clone()).id();
        entity_map.add_switch(spawn_event.switch.id, entity);
    }
}

pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSwitchEvent>();
        app.add_event::<UpdateSwitchTurnsEvent>();
        app.register_component_as::<dyn Selectable, Switch>();
        app.add_systems(
            Update,
            (
                spawn_switch.run_if(on_event::<SpawnSwitchEvent>()),
                update_switch_turns.run_if(on_event::<UpdateSwitchTurnsEvent>()),
                draw_switches,
            ),
        );
    }
}
