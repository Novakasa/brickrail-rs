use crate::{
    ble::BLEHub,
    editor::{GenericID, Selectable, SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_devices::{select_device_id, LayoutDevice, SpawnLayoutDeviceEvent},
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

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Default)]
struct SwitchMotor {
    hub_id: Option<HubID>,
    port: Option<HubPort>,
    #[serde(skip)]
    position: MotorPosition,
    #[serde(default)]
    inverted: bool,
    pulse_duration: f32,
    pulse_strength: f32,
}

impl SwitchMotor {
    pub fn inspector(
        &mut self,
        ui: &mut Ui,
        hubs: &Query<&BLEHub>,
        spawn_events: &mut EventWriter<SpawnHubEvent>,
        entity_map: &mut ResMut<EntityMap>,
        selection_state: &mut ResMut<SelectionState>,
    ) {
        BLEHub::select_port_ui(
            ui,
            &mut self.hub_id,
            &mut self.port,
            HubType::Layout,
            hubs,
            spawn_events,
            entity_map,
            selection_state,
        )
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
            EventWriter<SpawnLayoutDeviceEvent>,
            Query<&mut LayoutDevice>,
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
                            LayoutDeviceType::Switch,
                            &devices,
                            &mut spawn_devices,
                            &mut entity_map,
                        );
                        if let Some(motor_id) = motor_id {
                            let motor = devices
                                .get_mut(entity_map.layout_devices[motor_id])
                                .unwrap();
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

struct BLESwitchPlugin;

impl Plugin for BLESwitchPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Selectable, BLESwitch>();
    }
}
