use crate::{
    ble::BLEHub,
    editor::{GenericID, Selectable, SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_devices::{select_device_id, DeviceComponent, LayoutDevice, SpawnDeviceID},
    layout_primitives::*,
};
use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::egui::Ui;

use bevy_trait_query::RegisterExt as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Default)]
pub enum MotorPosition {
    #[default]
    Unknown,
    Left,
    Right,
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Default, Component)]
struct SwitchMotor {
    #[serde(skip)]
    position: MotorPosition,
    #[serde(default)]
    inverted: bool,
    pulse_duration: f32,
    pulse_strength: f32,
}

impl SwitchMotor {}

impl DeviceComponent for SwitchMotor {
    type SpawnEvent = SpawnSwitchMotorEvent;

    fn new_id(entity_map: &mut EntityMap) -> LayoutDeviceID {
        entity_map.new_layout_device_id(LayoutDeviceType::Switch)
    }
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Event)]
struct SpawnSwitchMotorEvent {
    device: LayoutDevice,
    motor: SwitchMotor,
}

impl SpawnDeviceID for SpawnSwitchMotorEvent {
    fn from_id(id: LayoutDeviceID) -> Self {
        Self {
            device: LayoutDevice::from_id(id),
            motor: SwitchMotor::default(),
        }
    }
}

fn spawn_switch_motor(
    mut events: EventReader<SpawnSwitchMotorEvent>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in events.read() {
        let entity = commands
            .spawn((event.device.clone(), event.motor.clone()))
            .id();
        entity_map.layout_devices.insert(event.device.id, entity);
        println!("Spawned switch motor with id {:?}", event.device.id);
    }
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct BLESwitch {
    id: DirectedTrackID,
    motors: Vec<Option<LayoutDeviceID>>,
}

impl BLESwitch {
    pub fn new(id: DirectedTrackID) -> Self {
        Self {
            id,
            motors: Vec::new(),
        }
    }

    pub fn set_num_motors(&mut self, num: usize) {
        self.motors.resize_with(num, Default::default);
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut BLESwitch>,
            ResMut<EntityMap>,
            ResMut<SelectionState>,
            Res<AppTypeRegistry>,
            Query<&BLEHub>,
            EventWriter<SpawnHubEvent>,
            EventWriter<SpawnSwitchMotorEvent>,
            Query<(&mut SwitchMotor, &mut LayoutDevice)>,
        )>::new(world);
        let (
            mut ble_switches,
            mut entity_map,
            mut selection_state,
            _type_registry,
            hubs,
            mut spawn_events,
            mut spawn_devices,
            mut devices,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut ble_switch) = ble_switches.get_mut(entity) {
                ui.label("BLE Switch");
                for (i, motor_id) in &mut ble_switch.motors.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.label(format!("Motor {:}", i));
                        select_device_id(
                            ui,
                            motor_id,
                            &mut devices,
                            &mut spawn_devices,
                            &mut entity_map,
                        );
                        if let Some(motor_id) = motor_id {
                            if let Some(entity) = entity_map.layout_devices.get(motor_id) {
                                if let Ok((motor, mut device)) = devices.get_mut(*entity) {
                                    device.inspector(
                                        ui,
                                        &hubs,
                                        &mut spawn_events,
                                        &mut entity_map,
                                        &mut selection_state,
                                    )
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

impl Selectable for BLESwitch {
    fn get_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }
}

pub struct BLESwitchPlugin;

impl Plugin for BLESwitchPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Selectable, BLESwitch>();
        app.add_event::<SpawnSwitchMotorEvent>();
        app.add_systems(
            Update,
            spawn_switch_motor.run_if(on_event::<SpawnSwitchMotorEvent>()),
        );
    }
}
