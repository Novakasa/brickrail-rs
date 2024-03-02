use crate::{
    ble::BLEHub,
    editor::{GenericID, Selectable, SelectionState, SpawnHubEvent},
    layout::EntityMap,
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
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct BLESwitch {
    id: DirectedTrackID,
    motors: Vec<SwitchMotor>,
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
        )>::new(world);
        let (
            mut ble_switches,
            mut entity_map,
            mut selection_state,
            _type_registry,
            hubs,
            mut spawn_events,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut ble_switch) = ble_switches.get_mut(entity) {
                ui.label("BLE Switch");
                for (i, motor) in &mut ble_switch.motors.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.label(format!("Motor {:}", i));
                        BLEHub::select_port_ui(
                            ui,
                            &mut motor.hub_id,
                            &mut motor.port,
                            HubType::Layout,
                            &hubs,
                            &mut spawn_events,
                            &mut entity_map,
                            &mut selection_state,
                        );
                        ui.checkbox(&mut motor.inverted, "Inverted");
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
