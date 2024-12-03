use crate::{
    ble::{HubCommandEvent, HubConfiguration},
    layout::EntityMap,
    layout_devices::{DeviceComponent, LayoutDevice, SpawnDeviceID},
    layout_primitives::*,
};
use bevy::{prelude::*, reflect::TypeRegistry, utils::hashbrown::HashMap};
use bevy_egui::egui::Ui;
use bevy_inspector_egui::bevy_egui;

use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions, reflect_inspector::ui_for_value, InspectorOptions,
};
use pybricks_ble::io_hub::Input;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Reflect, Serialize, Deserialize, Clone, Default, PartialEq, Eq, InspectorOptions,
)]
pub enum MotorPosition {
    #[default]
    Unknown,
    Left,
    Right,
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum MotorPolarity {
    #[default]
    Normal,
    Inverted,
}

impl MotorPolarity {
    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::Inverted => 1,
        }
    }
}

impl MotorPosition {
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Unknown => 2,
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Component, InspectorOptions)]
#[reflect(InspectorOptions)]
pub struct SwitchMotor {
    #[serde(skip)]
    pub position: MotorPosition,
    #[serde(default)]
    pub pulse_duration: u16,
    pub pulse_strength: u16,
    #[serde(default)]
    pub polarity: MotorPolarity,
}

impl Default for SwitchMotor {
    fn default() -> Self {
        Self {
            position: MotorPosition::Unknown,
            pulse_duration: 500,
            pulse_strength: 100,
            polarity: MotorPolarity::Normal,
        }
    }
}

impl SwitchMotor {
    pub fn inspector(&mut self, ui: &mut Ui, type_registry: &TypeRegistry) {
        ui_for_value(self, ui, type_registry);
    }

    pub fn switch_command(
        device: &LayoutDevice,
        position: &MotorPosition,
    ) -> Option<HubCommandEvent> {
        let input = Input::rpc(
            "device_execute",
            &vec![device.port?.to_u8(), 0, position.to_u8()],
        );
        Some(HubCommandEvent::input(device.hub_id?, input))
    }

    pub fn hub_configuration(&self, device: &LayoutDevice) -> HashMap<HubID, HubConfiguration> {
        if device.hub_id.is_none() {
            return HashMap::new();
        }

        let address_offset = 8 + device.port.unwrap().to_u8() * 16;
        let mut config = HubConfiguration::default();
        config.add_value(address_offset + 0, self.pulse_strength as u32);
        config.add_value(address_offset + 1, self.pulse_duration as u32);
        config.add_value(address_offset + 2, self.polarity.to_u32());

        let mut map = HashMap::new();
        map.insert(device.hub_id.unwrap(), config);
        map
    }
}

impl DeviceComponent for SwitchMotor {
    type SpawnEvent = SpawnSwitchMotorEvent;

    fn new_id(entity_map: &mut EntityMap) -> LayoutDeviceID {
        entity_map.new_layout_device_id(LayoutDeviceType::SwitchMotor)
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
            spawn_switch_motor.run_if(on_event::<SpawnSwitchMotorEvent>),
        );
    }
}
