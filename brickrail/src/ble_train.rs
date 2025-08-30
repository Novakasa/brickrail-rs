use std::iter;

use bevy::ecs::system::SystemState;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::egui::{self, Grid, Ui};
use bevy_inspector_egui::reflect_inspector::ui_for_value;
use itertools::Itertools;
use pybricks_ble::io_hub::{IOMessage, Input as IOInput};
use serde::{Deserialize, Serialize};

use crate::{
    ble::{BLEHub, FromIOMessage, HubCommandEvent, HubConfiguration, HubMessageEvent},
    editor::{SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_primitives::{Facing, HubID, HubPort, HubType, TrainID},
    marker::{MarkerColor, MarkerSpeed},
    route::{LegIntention, Route},
    train::{MarkerAdvanceEvent, Train},
};

#[derive(Debug)]
pub enum TrainData {
    RouteComplete(u8),
    LegAdvance(u8),
    SensorAdvance(u8),
    UnexpectedMarker {
        expected_color: MarkerColor,
        actual_color: MarkerColor,
        chroma: u16,
        hue: u16,
        samples: u16,
    },
    ReportDevices {
        has_sensor: bool,
        num_motors: u8,
    },
    Dump(u8, Vec<u8>),
}

impl FromIOMessage for TrainData {
    fn from_io_message(msg: &IOMessage) -> Option<Self> {
        match msg {
            IOMessage::Data { id, data } => match id {
                1 => Some(TrainData::RouteComplete(data[0])),
                2 => Some(TrainData::LegAdvance(data[0])),
                3 => Some(TrainData::SensorAdvance(data[0])),
                4 => Some(TrainData::UnexpectedMarker {
                    expected_color: MarkerColor::from_train_u8(data[0]).unwrap(),
                    actual_color: MarkerColor::from_train_u8(data[1]).unwrap(),
                    chroma: u16::from_be_bytes([data[2], data[3]]),
                    hue: u16::from_be_bytes([data[4], data[5]]),
                    samples: u16::from_be_bytes([data[6], data[7]]),
                }),
                5 => Some(TrainData::ReportDevices {
                    has_sensor: data[0] != 0,
                    num_motors: data[1],
                }),
                _ => None,
            },
            IOMessage::Sys { code, data } => panic!("Unhandled SysCode: {} {:?}", code, data),
            IOMessage::Dump { id, data } => Some(TrainData::Dump(*id, data.clone())),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TrainHub {
    pub hub_id: Option<HubID>,
    #[serde(default)]
    inverted_ports: Vec<HubPort>,
}

impl TrainHub {
    pub fn inspector_ui(
        &mut self,
        ui: &mut Ui,
        hubs: &Query<&BLEHub>,
        spawn_events: &mut EventWriter<SpawnHubEvent>,
        entity_map: &mut ResMut<EntityMap>,
        selection_state: &mut ResMut<SelectionState>,
        type_registry: &Res<AppTypeRegistry>,
    ) {
        ui.horizontal(|ui| {
            ui.label("Hub");
            BLEHub::select_id_ui(
                ui,
                &mut self.hub_id,
                HubType::Train,
                &hubs,
                spawn_events,
                entity_map,
                selection_state,
            );
        });
        ui.label("Inverted Ports");
        let mut remove_index = None;
        for (i, port) in self.inverted_ports.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    ui_for_value(port, ui, &type_registry.read());
                    if ui.button("Remove").clicked() {
                        remove_index = Some(i);
                        println!("Remove {}", i);
                    }
                });
            });
        }
        if let Some(i) = remove_index {
            self.inverted_ports.remove(i);
        }
        if ui.button("Add").clicked() {
            self.inverted_ports.push(HubPort::A);
        }
    }
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLETrain {
    pub master_hub: TrainHub,
    pub puppets: Vec<TrainHub>,
    pub train_id: TrainID,
    #[serde(default)]
    slow_speed: u16,
    #[serde(default)]
    cruise_speed: u16,
    #[serde(default)]
    fast_speed: u16,
    #[serde(default)]
    acceleration: u16,
    #[serde(default)]
    deceleration: u16,
    #[serde(default)]
    chroma_threshold: u16,
}

impl BLETrain {
    pub fn new(train_id: TrainID) -> Self {
        Self {
            master_hub: TrainHub::default(),
            puppets: Vec::new(),
            train_id,
            slow_speed: 40,
            cruise_speed: 70,
            fast_speed: 100,
            acceleration: 40,
            deceleration: 90,
            chroma_threshold: 3500,
        }
    }

    pub fn iter_puppets(&self) -> impl Iterator<Item = &HubID> {
        self.puppets.iter().filter_map(|id| id.hub_id.as_ref())
    }

    pub fn iter_all_hubs(&self) -> impl Iterator<Item = &HubID> {
        self.master_hub.hub_id.iter().chain(self.iter_puppets())
    }

    pub fn run_command(&self, facing: Facing, speed: MarkerSpeed) -> HubCommands {
        let arg: u8 = (facing.as_train_flag()) << 4 | speed.as_train_u8();
        let input = IOInput::rpc("run", &vec![arg]);
        self.all_command(input)
    }

    pub fn stop_command(&self) -> HubCommands {
        let input = IOInput::rpc("stop", &vec![]);
        self.all_command(input)
    }

    fn master_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        command.push(HubCommandEvent::input(
            self.master_hub.hub_id.unwrap(),
            input,
        ));
        command
    }

    pub fn download_route(&self, route: &Route) -> HubCommands {
        let input = IOInput::rpc("new_route", &vec![]);
        let mut command = self.all_command(input);
        for (i, leg) in route.iter_legs().enumerate() {
            let mut args = vec![i as u8];
            args.extend(leg.as_train_data());
            let input = IOInput::rpc("set_route_leg", &args);
            command.merge(self.all_command(input));
        }
        command
    }

    pub fn set_leg_intention(&self, leg_index: u8, intention: LegIntention) -> HubCommands {
        let args = vec![leg_index, intention.as_train_flag()];
        let input = IOInput::rpc("set_leg_intention", &args);
        self.all_command(input)
    }

    pub fn advance_sensor(&self) -> HubCommands {
        let input = IOInput::rpc("advance_sensor", &vec![]);
        self.puppet_command(input)
    }

    fn puppet_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        for hub in self.iter_puppets() {
            command.push(HubCommandEvent::input(*hub, input.clone()));
        }
        command
    }

    fn all_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        if let Some(hub_id) = self.master_hub.hub_id {
            command.push(HubCommandEvent::input(hub_id, input.clone()));
        }
        for hub in self.puppets.iter().filter_map(|id| id.hub_id.as_ref()) {
            command.push(HubCommandEvent::input(*hub, input.clone()));
        }
        command
    }

    pub fn hubs_configuration(&self) -> HashMap<HubID, HubConfiguration> {
        let mut configs = HashMap::default();
        for hub in iter::once(&self.master_hub).chain(self.puppets.iter()) {
            let mut config = HubConfiguration::default();
            config.add_value(4, self.slow_speed as u32);
            config.add_value(5, self.cruise_speed as u32);
            config.add_value(3, self.fast_speed as u32);
            config.add_value(1, self.acceleration as u32);
            config.add_value(2, self.deceleration as u32);
            config.add_value(0, self.chroma_threshold as u32);
            for port in HubPort::iter() {
                let inverted = hub.inverted_ports.contains(&port) as u32;
                if inverted != 0 {
                    config.add_value(6 + port.to_u8(), inverted);
                }
            }
            if let Some(hub_id) = hub.hub_id {
                configs.insert(hub_id, config);
            }
        }
        configs
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut BLETrain>,
            ResMut<EntityMap>,
            ResMut<SelectionState>,
            Res<AppTypeRegistry>,
            Query<&BLEHub>,
            EventWriter<SpawnHubEvent>,
        )>::new(world);
        let (
            mut ble_trains,
            mut entity_map,
            mut selection_state,
            type_registry,
            hubs,
            mut spawn_events,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut ble_train) = ble_trains.get_mut(entity) {
                ui.heading("Master Hub");
                ble_train.master_hub.inspector_ui(
                    ui,
                    &hubs,
                    &mut spawn_events,
                    &mut entity_map,
                    &mut selection_state,
                    &type_registry,
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.heading("Puppet hubs");
                    if ui.button("Add hub").clicked() {
                        ble_train.puppets.push(TrainHub::default());
                    }
                });
                ui.separator();
                let mut remove_index = None;
                for (i, hub) in ble_train.puppets.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("Puppet {:?}", i));
                            if ui.button("Remove").clicked() {
                                remove_index = Some(i);
                            }
                        });

                        hub.inspector_ui(
                            ui,
                            &hubs,
                            &mut spawn_events,
                            &mut entity_map,
                            &mut selection_state,
                            &type_registry,
                        );

                        ui.separator();
                    });
                }
                if let Some(i) = remove_index {
                    ble_train.puppets.remove(i);
                }

                ui.heading("Speeds");
                Grid::new("speeds").show(ui, |ui| {
                    ui.label("Slow");
                    ui.add(egui::Slider::new(&mut ble_train.slow_speed, 0..=100));
                    ui.end_row();
                    ui.label("Cruise");
                    ui.add(egui::Slider::new(&mut ble_train.cruise_speed, 0..=100));
                    ui.end_row();
                    ui.label("Fast");
                    ui.add(egui::Slider::new(&mut ble_train.fast_speed, 0..=100));
                    ui.end_row();
                    ui.label("Acceleration");
                    ui.add(egui::DragValue::new(&mut ble_train.acceleration));
                    ui.end_row();
                    ui.label("Deceleration");
                    ui.add(egui::DragValue::new(&mut ble_train.deceleration));
                    ui.end_row();
                    ui.label("Chroma Threshold");
                    ui.add(egui::DragValue::new(&mut ble_train.chroma_threshold));
                    ui.end_row();
                });
            }
        }
    }
}

pub struct HubCommands {
    pub hub_events: Vec<HubCommandEvent>,
}

impl HubCommands {
    fn new() -> Self {
        Self {
            hub_events: Vec::new(),
        }
    }

    fn push(&mut self, hub_input: HubCommandEvent) {
        self.hub_events.push(hub_input);
    }

    fn merge(&mut self, mut other: HubCommands) {
        self.hub_events.append(&mut other.hub_events);
    }
}

fn handle_messages(
    mut hub_message_events: EventReader<HubMessageEvent<TrainData>>,
    mut ble_trains: Query<(&BLETrain, &mut Train)>,
    mut advance_events: EventWriter<MarkerAdvanceEvent>,
    mut ble_commands: EventWriter<HubCommandEvent>,
) {
    for event in hub_message_events.read() {
        for (ble_train, _train) in ble_trains.iter_mut() {
            if ble_train.master_hub.hub_id == Some(event.id) {
                match event.data {
                    TrainData::ReportDevices {
                        has_sensor,
                        num_motors: _,
                    } => {
                        if !has_sensor {
                            error!("Train master hub {:?} has no sensor", event.id);
                        }
                    }
                    TrainData::LegAdvance(index) => {
                        info!("Train master hub {:?} leg advance: {}", event.id, index);
                        // :train.get_route_mut().next_leg().unwrap();
                    }
                    TrainData::SensorAdvance(index) => {
                        info!("Train master hub {:?} sensor advance: {}", event.id, index);
                        advance_events.write(MarkerAdvanceEvent {
                            id: ble_train.train_id,
                            index: index as usize,
                        });
                        for input in ble_train.advance_sensor().hub_events {
                            ble_commands.write(input);
                        }
                    }
                    _ => warn!("Unhandled TrainData: {:?}", event.data),
                }
            }
            if ble_train
                .puppets
                .iter()
                .map(|hub| hub.hub_id)
                .collect_vec()
                .contains(&Some(event.id))
            {
                match event.data {
                    TrainData::ReportDevices {
                        has_sensor,
                        num_motors: _,
                    } => {
                        if has_sensor {
                            error!("Train puppet hub {:?} has sensor", event.id);
                        }
                    }
                    TrainData::SensorAdvance(index) => {
                        error!(
                            "Train puppet hub {:?} sensor advance event: {}",
                            event.id, index
                        );
                    }
                    _ => warn!("Unhandled TrainData for puppet: {:?}", event.data),
                }
            }
        }
    }
}

pub struct BLETrainPlugin;

impl Plugin for BLETrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HubMessageEvent<TrainData>>();
        app.add_event::<MarkerAdvanceEvent>();
        app.add_systems(
            Update,
            handle_messages.run_if(on_event::<HubMessageEvent<TrainData>>),
        );
    }
}
