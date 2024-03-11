use crate::{
    layout::EntityMap,
    layout_devices::{DeviceComponent, LayoutDevice, SpawnDeviceID},
    layout_primitives::*,
};
use bevy::{prelude::*, reflect::TypeRegistry};
use bevy_egui::egui::Ui;

use bevy_inspector_egui::reflect_inspector::ui_for_value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Default)]
pub enum MotorPosition {
    #[default]
    Unknown,
    Left,
    Right,
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Default, Component)]
pub struct SwitchMotor {
    #[serde(skip)]
    position: MotorPosition,
    #[serde(default)]
    inverted: bool,
    pulse_duration: f32,
    pulse_strength: f32,
}

impl SwitchMotor {
    pub fn inspector(&mut self, ui: &mut Ui, type_registry: &TypeRegistry) {
        ui_for_value(self, ui, type_registry);
    }
}

impl DeviceComponent for SwitchMotor {
    type SpawnEvent = SpawnSwitchMotorEvent;

    fn new_id(entity_map: &mut EntityMap) -> LayoutDeviceID {
        entity_map.new_layout_device_id(LayoutDeviceType::Switch)
    }
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Event)]
pub struct SpawnSwitchMotorEvent {
    pub device: LayoutDevice,
    pub motor: SwitchMotor,
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

pub struct SwitchMotorPlugin;

impl Plugin for SwitchMotorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSwitchMotorEvent>();
        app.add_systems(
            Update,
            spawn_switch_motor.run_if(on_event::<SpawnSwitchMotorEvent>()),
        );
    }
}
