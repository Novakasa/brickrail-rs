use bevy::prelude::*;
use bevy_egui::egui::{Align, Layout, Ui};
use bevy_trait_query::RegisterExt;
use pybricks_ble::io_hub::Input as IOInput;
use serde::{Deserialize, Serialize};

use crate::{
    ble::HubInput,
    editor::{GenericID, Selectable},
    inspector::InspectorContext,
    layout_primitives::{Facing, HubID, HubType, TrainID},
    marker::MarkerSpeed,
    route::Route,
};

#[derive(Component, Serialize, Deserialize)]
pub struct BLETrain {
    master_hub: Option<HubID>,
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
        command.push(HubInput::new(self.master_hub.unwrap(), input));
        command
    }

    pub fn download_route(&self, route: &Route) -> HubCommands {
        let input = IOInput::rpc("new_route", &vec![]);
        let mut command = self.all_command(input);
        for (i, leg) in route.iter_legs().enumerate() {
            let mut args = vec![i as u8];
            args.extend(leg.as_train_data());
            let input = IOInput::rpc("add_leg", &args);
            command.merge(self.all_command(input));
        }
        command
    }

    fn puppet_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        for hub_id in self.puppets.iter().filter_map(|id| id.as_ref()) {
            command.push(HubInput::new(*hub_id, input.clone()));
        }
        command
    }

    fn all_command(&self, input: IOInput) -> HubCommands {
        let mut command = HubCommands::new();
        command.push(HubInput::new(self.master_hub.unwrap(), input.clone()));
        for hub_id in self.puppets.iter().filter_map(|id| id.as_ref()) {
            command.push(HubInput::new(*hub_id, input.clone()));
        }
        command
    }
}

impl Selectable for BLETrain {
    fn get_id(&self) -> GenericID {
        GenericID::Train(self.train_id)
    }

    fn inspector_ui(&mut self, ui: &mut Ui, context: &mut InspectorContext) {
        ui.label("BLE Train");
        context.select_hub_ui(ui, &mut self.master_hub, HubType::Train);
        ui.label("Puppets");
        let mut remove_index = None;
        for (i, hub_id) in self.puppets.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    context.select_hub_ui(ui, hub_id, HubType::Train);
                    if ui.button("Remove").clicked() {
                        remove_index = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove_index {
            self.puppets.remove(i);
        }
        if ui.button("Add Puppet").clicked() {
            self.puppets.push(None);
        }
    }
}

pub struct HubCommands {
    hub_events: Vec<HubInput>,
}

impl HubCommands {
    fn new() -> Self {
        Self {
            hub_events: Vec::new(),
        }
    }

    fn push(&mut self, hub_input: HubInput) {
        self.hub_events.push(hub_input);
    }

    fn merge(&mut self, mut other: HubCommands) {
        self.hub_events.append(&mut other.hub_events);
    }
}

pub struct BLETrainPlugin;

impl Plugin for BLETrainPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Selectable, BLETrain>();
    }
}
