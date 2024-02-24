use bevy::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_egui::egui::{Align, Layout, Ui};
use bevy_trait_query::RegisterExt;
use pybricks_ble::io_hub::{IOMessage, Input as IOInput};
use serde::{Deserialize, Serialize};

use crate::{
    ble::{BLEHub, FromIOMessage, HubCommandEvent, HubMessageEvent},
    editor::{GenericID, Selectable, SelectionState, SerializedHub, SpawnEvent},
    layout::EntityMap,
    layout_primitives::{Facing, HubID, HubType, TrainID},
    marker::{MarkerColor, MarkerSpeed},
    route::{LegIntention, Route},
    train::Train,
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

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLETrain {
    pub master_hub: Option<HubID>,
    puppets: Vec<Option<HubID>>,
    train_id: TrainID,
}

impl BLETrain {
    pub fn new(train_id: TrainID) -> Self {
        Self {
            master_hub: None,
            puppets: Vec::new(),
            train_id,
        }
    }

    pub fn iter_puppets(&self) -> impl Iterator<Item = &HubID> {
        self.puppets.iter().filter_map(|id| id.as_ref())
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
        command.push(HubCommandEvent::input(self.master_hub.unwrap(), input));
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
        for hub_id in self.iter_puppets() {
            command.push(HubCommandEvent::input(*hub_id, input.clone()));
        }
        command
    }

    fn all_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        command.push(HubCommandEvent::input(
            self.master_hub.unwrap(),
            input.clone(),
        ));
        for hub_id in self.puppets.iter().filter_map(|id| id.as_ref()) {
            command.push(HubCommandEvent::input(*hub_id, input.clone()));
        }
        command
    }

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut BLETrain>,
            ResMut<EntityMap>,
            ResMut<SelectionState>,
            Res<AppTypeRegistry>,
            Query<&BLEHub>,
            EventWriter<SpawnEvent<SerializedHub>>,
        )>::new(world);
        let (
            mut ble_trains,
            mut entity_map,
            mut selection_state,
            _type_registry,
            hubs,
            mut spawn_events,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut ble_train) = ble_trains.get_mut(entity) {
                ui.label("BLE Train");
                ui.label("Master Hub");
                BLEHub::select_id_ui(
                    ui,
                    &mut ble_train.master_hub,
                    HubType::Train,
                    &hubs,
                    &mut spawn_events,
                    &mut entity_map,
                    &mut selection_state,
                );
                ui.label("Puppets");
                let mut remove_index = None;
                for (i, hub_id) in ble_train.puppets.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            BLEHub::select_id_ui(
                                ui,
                                hub_id,
                                HubType::Train,
                                &hubs,
                                &mut spawn_events,
                                &mut entity_map,
                                &mut selection_state,
                            );
                            if ui.button("Remove").clicked() {
                                remove_index = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove_index {
                    ble_train.puppets.remove(i);
                }
                if ui.button("Add Puppet").clicked() {
                    ble_train.puppets.push(None);
                }
                ui.separator();
            }
        }
    }
}

impl Selectable for BLETrain {
    fn get_id(&self) -> GenericID {
        GenericID::Train(self.train_id)
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

#[derive(Event)]
pub struct BLESensorAdvanceEvent {
    pub id: TrainID,
    pub index: u8,
}

fn handle_messages(
    mut hub_message_events: EventReader<HubMessageEvent<TrainData>>,
    mut ble_trains: Query<(&BLETrain, &mut Train)>,
    mut advance_events: EventWriter<BLESensorAdvanceEvent>,
    mut ble_commands: EventWriter<HubCommandEvent>,
) {
    for event in hub_message_events.read() {
        for (ble_train, mut train) in ble_trains.iter_mut() {
            if ble_train.master_hub == Some(event.id) {
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
                        train.get_route_mut().next_leg();
                    }
                    TrainData::SensorAdvance(index) => {
                        info!("Train master hub {:?} sensor advance: {}", event.id, index);
                        advance_events.send(BLESensorAdvanceEvent {
                            id: ble_train.train_id,
                            index,
                        });
                        for input in ble_train.advance_sensor().hub_events {
                            ble_commands.send(input);
                        }
                    }
                    _ => warn!("Unhandled TrainData: {:?}", event.data),
                }
            }
            if ble_train.puppets.contains(&Some(event.id)) {
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
        app.register_component_as::<dyn Selectable, BLETrain>();
        app.add_event::<HubMessageEvent<TrainData>>();
        app.add_event::<BLESensorAdvanceEvent>();
        app.add_systems(
            Update,
            handle_messages.run_if(on_event::<HubMessageEvent<TrainData>>()),
        );
    }
}
