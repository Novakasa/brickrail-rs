use std::{path::Path, sync::Arc};

use crate::{
    bevy_tokio_tasks::TokioTasksRuntime,
    ble_train::{BLETrain, TrainData},
    editor::{
        DespawnEvent, EditorState, GenericID, Selection, SelectionState, SpawnHubEvent,
        delete_selection_shortcut,
    },
    inspector::{Inspectable, InspectorPlugin},
    layout::EntityMap,
    layout_devices::LayoutDevice,
    layout_primitives::{HubID, HubPort, HubType},
    selectable::{Selectable, SelectablePlugin, SelectableType},
    settings::Settings,
    switch::Switch,
    switch_motor::PulseMotor,
};
use bevy::{ecs::system::SystemState, platform::collections::HashMap};
use bevy::{input::keyboard, prelude::*};
use bevy_inspector_egui::bevy_egui::egui::{self, Grid, Ui, widgets::Button};
use pybricks_ble::io_hub::{IOEvent, IOHub, IOMessage, Input as IOInput, SysCode};
use pybricks_ble::pybricks_hub::HubStatusFlags;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc::UnboundedSender};

#[derive(Clone, Default, Debug, PartialEq)]
pub enum HubState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Downloading(f32),
    StartingProgram,
    Running,
    Configuring,
    Ready,
    StoppingProgram,
    Disconnecting,
}

impl HubState {
    pub fn is_running(&self) -> bool {
        match self {
            HubState::Running | HubState::Configuring | HubState::Ready => true,
            _ => false,
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            HubState::Disconnected | HubState::Connecting => false,
            _ => true,
        }
    }

    pub fn is_busy(&self) -> bool {
        match self {
            HubState::Connecting
            | HubState::Downloading(_)
            | HubState::StartingProgram
            | HubState::Configuring
            | HubState::StoppingProgram
            | HubState::Disconnecting => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HubError {
    ConnectError,
    ProgramError,
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLEHub {
    pub id: HubID,
    #[serde(skip)]
    hub: Arc<Mutex<IOHub>>,
    #[serde(skip)]
    input_sender: Option<UnboundedSender<IOInput>>,
    pub name: Option<String>,
    #[serde(skip)]
    pub active: bool,
    #[serde(skip)]
    pub state: HubState,
    #[serde(skip)]
    pub error: Option<HubError>,
    #[serde(skip)]
    downloaded: bool,
    #[serde(skip)]
    config: HubConfiguration,
}

impl BLEHub {
    pub fn new(id: HubID) -> Self {
        Self {
            id,
            hub: Arc::new(Mutex::new(IOHub::new())),
            input_sender: None,
            name: None,
            active: false,
            state: HubState::Disconnected,
            error: None,
            downloaded: false,
            config: HubConfiguration::default(),
        }
    }

    pub fn get_program_path(&self) -> &'static Path {
        // print cwd:
        println!("{:?}", std::env::current_dir().unwrap());
        match self.id.kind {
            HubType::Layout => Path::new("pybricks/programs/mpy/layout_controller.mpy"),
            HubType::Train => Path::new("pybricks/programs/mpy/smart_train.mpy"),
        }
    }

    pub fn get_program_hash(&self) -> String {
        let path = self.get_program_path();
        crate::utils::get_file_hash(path)
    }

    pub fn set_downloaded_from_settings(&mut self, settings: &Res<Settings>) {
        self.downloaded = if let Some(name) = self.name.as_ref() {
            let hash = self.get_program_hash();
            match settings.program_hashes.get(name) {
                Some(h) => h == &hash,
                None => false,
            }
        } else {
            false
        };
    }

    pub fn sync_settings_hash(&mut self, settings: &mut ResMut<Settings>) {
        if let Some(name) = self.name.as_ref() {
            let hash = self.get_program_hash();
            settings.program_hashes.insert(name.clone(), hash);
        }
    }
}

impl Inspectable for BLEHub {
    fn inspector(ui: &mut Ui, world: &mut World) {
        BLEHub::inspector(ui, world);
    }

    fn run_condition(selection_state: Res<SelectionState>) -> bool {
        selection_state.selected_type() == Some(SelectableType::Hub)
    }
}

impl Selectable for BLEHub {
    type SpawnEvent = SpawnHubEvent;
    type ID = HubID;

    fn get_type() -> SelectableType {
        SelectableType::Hub
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Hub(self.id)
    }

    fn default_spawn_event(entity_map: &mut ResMut<EntityMap>) -> Option<Self::SpawnEvent> {
        Some(SpawnHubEvent {
            hub: BLEHub::new(entity_map.new_hub_id(HubType::Layout)),
        })
    }
    fn id(&self) -> Self::ID {
        self.id.clone()
    }
}

impl BLEHub {
    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&BLEHub>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            EventWriter<HubCommandEvent>,
        )>::new(world);
        let (hubs, entity_map, selection_state, _type_registry, mut command_events) =
            state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(hub) = hubs.get(entity) {
                ui.label(format!("BLE Hub {:?}", hub.id));
                ui.label(format!(
                    "Name: {}",
                    hub.name.as_deref().unwrap_or("Unknown")
                ));
                ui.label(format!("State: {:?}", hub.state));
                if ui
                    .button("Discover Name")
                    .on_hover_text("Discover the name of the hub")
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::DiscoverName,
                    });
                }
                if ui
                    .add_enabled(
                        hub.name.is_some() && hub.state == HubState::Disconnected,
                        Button::new("Connect"),
                    )
                    .on_hover_text("Connect to the hub")
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::Connect,
                    });
                }
                if ui
                    .add_enabled(hub.state == HubState::Connected, Button::new("Disconnect"))
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::Disconnect,
                    });
                }
                if ui
                    .add_enabled(
                        hub.state == HubState::Connected,
                        Button::new("Download Program"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::DownloadProgram,
                    });
                }
                if ui
                    .add_enabled(
                        hub.state == HubState::Connected,
                        Button::new("Start Program"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::StartProgram,
                    });
                }
                if ui
                    .add_enabled(hub.state == HubState::Running, Button::new("Stop Program"))
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.write(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::StopProgram,
                    });
                }
                ui.separator();
            }
        }
    }

    pub fn select_port_ui(
        ui: &mut Ui,
        selected_hub: &mut Option<HubID>,
        selected_port: &mut Option<HubPort>,
        kind: HubType,
        hubs: &Query<&BLEHub>,
        spawn_events: &mut EventWriter<SpawnHubEvent>,
        entity_map: &mut ResMut<EntityMap>,
        selection_state: &mut ResMut<SelectionState>,
    ) {
        Grid::new("port select").show(ui, |ui| {
            ui.label("Hub");
            ui.push_id("motor", |ui| {
                Self::select_id_ui(
                    ui,
                    selected_hub,
                    kind,
                    hubs,
                    spawn_events,
                    entity_map,
                    selection_state,
                );
            });
            if selected_hub.is_none() && selected_port.is_some() {
                *selected_port = None;
            }
            ui.end_row();
            ui.label("Port");
            ui.add_enabled_ui(selected_hub.is_some(), |ui| {
                ui.push_id("port", |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_label("")
                            .selected_text(format!(
                                "{:}",
                                selected_port
                                    .map(|h| h.to_string())
                                    .unwrap_or("None".to_string())
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(selected_port, None, "None");
                                for option in HubPort::iter() {
                                    ui.selectable_value(
                                        selected_port,
                                        Some(option),
                                        option.to_string(),
                                    );
                                }
                            });
                    });
                });
            });
        });
    }

    pub fn select_id_ui(
        ui: &mut Ui,
        selected: &mut Option<HubID>,
        kind: HubType,
        hubs: &Query<&BLEHub>,
        spawn_events: &mut EventWriter<SpawnHubEvent>,
        entity_map: &mut ResMut<EntityMap>,
        selection_state: &mut ResMut<SelectionState>,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("")
                .selected_text(match selected {
                    Some(id) => get_hub_label(hubs, id),
                    None => "None".to_string(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected, None, "None");
                    for hub in hubs.iter().filter(|hub| hub.id.kind == kind) {
                        ui.selectable_value(
                            selected,
                            Some(hub.id.clone()),
                            get_hub_label(hubs, &hub.id),
                        );
                    }
                    if ui
                        .button("New Hub")
                        .on_hover_text("Create a new hub")
                        .clicked()
                    {
                        *selected = Some(entity_map.new_hub_id(kind));
                        let hub = BLEHub::new(selected.unwrap().clone());
                        spawn_events.write(SpawnHubEvent { hub });
                    };
                });
            if let Some(hub_id) = selected {
                if ui.button("edit").clicked() {
                    selection_state.selection = Selection::Single(GenericID::Hub(hub_id.clone()));
                }
            }
        });
    }
}

fn get_hub_label(hubs: &Query<&BLEHub>, id: &HubID) -> String {
    for hub in hubs.iter() {
        if &hub.id == id {
            return match hub.name.as_ref() {
                Some(name) => name.clone(),
                None => format!("Unkown {:}", id),
            };
        }
    }
    return format!("Unkown {:}", id);
}

fn create_hub(
    mut hub_event_writer: EventWriter<SpawnHubEvent>,
    keyboard_input: Res<ButtonInput<keyboard::KeyCode>>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::KeyH) {
        let id = entity_map.new_hub_id(HubType::Layout);
        let hub = BLEHub::new(id);
        hub_event_writer.write(SpawnHubEvent { hub });
    }
}

fn spawn_hub(
    runtime: Res<TokioTasksRuntime>,
    mut spawn_event_reader: EventReader<SpawnHubEvent>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    settings: Res<Settings>,
) {
    for event in spawn_event_reader.read() {
        let mut hub = event.hub.clone();
        hub.set_downloaded_from_settings(&settings);
        println!("name: {:?}", hub.name);
        println!("downloaded: {:?}", hub.downloaded);
        let hub_id = hub.id;
        let hub_mutex = hub.hub.clone();
        let name = Name::new(hub.name.clone().unwrap_or(hub_id.to_string()));
        let entity = commands.spawn((name, hub)).id();
        entity_map.add_hub(hub_id, entity);

        runtime.spawn_background_task(move |mut ctx| async move {
            let mut event_receiver = hub_mutex.lock().await.subscribe_events();
            println!("Listening for events on hub {:?}", hub_id);
            while let Ok(event) = event_receiver.recv().await {
                ctx.run_on_main_thread(move |ctx| {
                    ctx.world.send_event(HubEvent {
                        hub_id,
                        event: event,
                    })
                })
                .await;
            }
        });
    }
}

fn despawn_hub(
    mut hub_event_reader: EventReader<DespawnEvent<BLEHub>>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    mut q_ble_trains: Query<&mut BLETrain>,
    mut q_layout_devices: Query<&mut LayoutDevice>,
) {
    for event in hub_event_reader.read() {
        for mut ble_train in q_ble_trains.iter_mut() {
            if let Some(master_hub) = ble_train.master_hub.hub_id.clone() {
                if master_hub == event.0 {
                    ble_train.master_hub.hub_id = None;
                }
            }
            for puppet in ble_train.puppets.iter_mut() {
                if Some(event.0) == puppet.hub_id {
                    puppet.hub_id = None;
                }
            }
        }

        for mut layout_device in q_layout_devices.iter_mut() {
            if let Some(hub_id) = layout_device.hub_id {
                if hub_id == event.0 {
                    layout_device.hub_id = None;
                }
            }
        }

        if let Some(entity) = entity_map.hubs.remove(&event.0) {
            commands.entity(entity).despawn();
        }
        entity_map.remove_hub(event.0);
    }
}

#[derive(Debug, Clone, Default)]
pub struct HubConfiguration {
    data: HashMap<u8, u32>,
}

impl HubConfiguration {
    pub fn add_value(&mut self, port: u8, value: u32) {
        self.data.insert(port, value);
    }

    pub fn merge(&mut self, other: &Self) {
        for (port, value) in other.data.iter() {
            self.data.insert(*port, *value);
        }
    }
}

#[derive(Event, Debug, Clone)]
pub enum HubCommand {
    DiscoverName,
    Connect,
    Disconnect,
    DownloadProgram,
    StartProgram,
    StopProgram,
    QueueInput(IOInput),
    Configure,
}

#[derive(Event, Debug)]
pub struct HubCommandEvent {
    pub hub_id: HubID,
    pub command: HubCommand,
}

impl HubCommandEvent {
    pub fn input(hub_id: HubID, input: IOInput) -> Self {
        Self {
            hub_id,
            command: HubCommand::QueueInput(input),
        }
    }
}

fn execute_hub_commands(
    mut hub_command_reader: EventReader<HubCommandEvent>,
    mut q_hubs: Query<&mut BLEHub>,
    entity_map: Res<EntityMap>,
    runtime: Res<TokioTasksRuntime>,
) {
    for event in hub_command_reader.read() {
        let entity = entity_map.hubs[&event.hub_id];
        let mut hub = q_hubs.get_mut(entity).unwrap();
        match event.command.clone() {
            HubCommand::DiscoverName => {
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.discover_name().await.unwrap();
                });
            }
            HubCommand::Connect => {
                hub.state = HubState::Connecting;
                let io_hub = hub.hub.clone();
                let name = hub.name.as_ref().unwrap().clone();
                runtime.spawn_background_task(move |mut ctx| async move {
                    if io_hub.lock().await.connect(&name).await.is_err() {
                        ctx.run_on_main_thread(move |ctx_main| {
                            let mut system_state: SystemState<(Query<&mut BLEHub>,)> =
                                SystemState::new(ctx_main.world);
                            let mut query = system_state.get_mut(ctx_main.world);
                            let mut hub = query.0.get_mut(entity).unwrap();
                            hub.error = Some(HubError::ConnectError);
                            hub.state = HubState::Disconnected;
                        })
                        .await;
                    }
                });
            }
            HubCommand::Disconnect => {
                hub.state = HubState::Disconnecting;
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |mut ctx| async move {
                    io_hub.lock().await.disconnect().await.unwrap();
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<(Query<&mut BLEHub>,)> =
                            SystemState::new(ctx_main.world);
                        let mut query = system_state.get_mut(ctx_main.world);
                        let mut hub = query.0.get_mut(entity).unwrap();
                        hub.state = HubState::Disconnected;
                    })
                    .await;
                });
            }
            HubCommand::DownloadProgram => {
                hub.state = HubState::Downloading(0.0);
                let io_hub = hub.hub.clone();
                let program = hub.get_program_path();
                runtime.spawn_background_task(move |mut ctx| async move {
                    io_hub.lock().await.download_program(program).await.unwrap();
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<(Query<&mut BLEHub>, ResMut<Settings>)> =
                            SystemState::new(ctx_main.world);
                        let (mut query, mut settings) = system_state.get_mut(ctx_main.world);
                        let mut hub = query.get_mut(entity).unwrap();
                        hub.downloaded = true;
                        hub.state = HubState::Connected;
                        hub.sync_settings_hash(&mut settings);
                    })
                    .await;
                });
            }
            HubCommand::StartProgram => {
                hub.state = HubState::StartingProgram;
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |mut ctx| async move {
                    let mut hub_mut = io_hub.lock().await;
                    hub_mut.start_program().await.unwrap();
                    let input_sender = hub_mut.get_input_queue_sender();
                    assert!(input_sender.is_some());
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<(Query<&mut BLEHub>,)> =
                            SystemState::new(ctx_main.world);
                        let mut query = system_state.get_mut(ctx_main.world);
                        let mut hub = query.0.get_mut(entity).unwrap();
                        hub.input_sender = input_sender;
                    })
                    .await;
                });
            }
            HubCommand::StopProgram => {
                hub.state = HubState::StoppingProgram;
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.stop_program().await.unwrap();
                });
            }
            HubCommand::QueueInput(input) => {
                hub.input_sender.as_ref().unwrap().send(input).unwrap();
            }
            HubCommand::Configure => {
                hub.state = HubState::Configuring;
                let config = hub.config.clone();
                let sender = hub.input_sender.as_ref().unwrap();
                for (adress, value) in config.data.iter() {
                    sender.send(IOInput::store_uint(*adress, *value)).unwrap();
                }
                sender.send(IOInput::sys(SysCode::Ready, &[])).unwrap();
            }
        }
    }
}

pub trait FromIOMessage: Sized {
    fn from_io_message(msg: &IOMessage) -> Option<Self>;
}

#[derive(Debug)]
pub enum SysData {
    Stop,
    Ready,
    Alive { voltage: f32, current: f32 },
    Version(String),
}

impl FromIOMessage for SysData {
    fn from_io_message(msg: &IOMessage) -> Option<Self> {
        match msg {
            IOMessage::Sys { code, data } => match code {
                0x00 => Some(SysData::Stop),
                0x01 => Some(SysData::Ready),
                0x02 => Some(SysData::Alive {
                    voltage: u16::from_be_bytes([data[0], data[1]]) as f32 / 1000.0,
                    current: u16::from_be_bytes([data[2], data[3]]) as f32 / 1000.0,
                }),
                0x03 => match std::str::from_utf8(data) {
                    Ok(version) => Some(SysData::Version(version.to_string())),
                    Err(_) => None,
                },
                _ => None,
            },
            _ => None,
        }
    }
}

#[derive(Event, Debug)]
pub struct HubMessageEvent<T: FromIOMessage> {
    pub id: HubID,
    pub data: T,
}

fn handle_hub_events(
    mut hub_event_reader: EventReader<HubEvent>,
    mut train_sender: EventWriter<HubMessageEvent<TrainData>>,
    mut q_hubs: Query<(&mut BLEHub, &mut Name)>,
    entity_map: Res<EntityMap>,
    settings: Res<Settings>,
) {
    for event in hub_event_reader.read() {
        let (mut hub, mut name_component) = q_hubs.get_mut(entity_map.hubs[&event.hub_id]).unwrap();
        match &event.event {
            IOEvent::NameDiscovered(name) => {
                hub.name = Some(name.clone());
                name_component.set(name.clone());
                hub.set_downloaded_from_settings(&settings);
                return;
            }
            IOEvent::Message(msg) => {
                debug!("Message: {:?}", msg);
                match msg {
                    IOMessage::Sys { code, data } => {
                        let data = SysData::from_io_message(msg).expect(
                            &format!(
                                "Could not parse SysData with code: {:?} data: {:?}",
                                code, data,
                            )
                            .to_string(),
                        );
                        info!("Received SysData: {:?}", data);
                        match data {
                            SysData::Ready => {
                                hub.state = HubState::Ready;
                            }
                            _ => {}
                        }
                    }
                    _ => match hub.id.kind {
                        HubType::Train => {
                            if let Some(data) = TrainData::from_io_message(msg) {
                                debug!("sending TrainData: {:?}", data);
                                train_sender.write(HubMessageEvent { id: hub.id, data });
                            }
                        }
                        _ => {
                            info!(
                                "Unhandled message for hub kind: {:?} {:?}",
                                hub.id.kind, msg
                            );
                        }
                    },
                }
            }
            IOEvent::Status(status) => {
                debug!("Status: {:?}", status);
                let running_flag = status.flags.clone() & HubStatusFlags::PROGRAM_RUNNING
                    == HubStatusFlags::PROGRAM_RUNNING;
                if running_flag {
                    if hub.state == HubState::StartingProgram {
                        hub.state = HubState::Running;
                    }
                } else {
                    match hub.state {
                        HubState::Running | HubState::Configuring | HubState::Ready => {
                            hub.state = HubState::Connected;
                            hub.error = Some(HubError::ProgramError);
                        }
                        HubState::StoppingProgram | HubState::Connecting => {
                            hub.state = HubState::Connected;
                        }
                        _ => {}
                    }
                }
            }
            IOEvent::DownloadProgress(progress) => {
                info!("Download progress: {:?}", progress);
                hub.state = HubState::Downloading(*progress);
            }
        }
    }
}

#[derive(Event, Debug)]
struct HubEvent {
    hub_id: HubID,
    event: IOEvent,
}

pub fn prepare_hubs(
    q_hubs: Query<&BLEHub>,
    mut command_events: EventWriter<HubCommandEvent>,
    mut editor_state: ResMut<NextState<EditorState>>,
) {
    let mut prepared = true;
    for hub in q_hubs.iter() {
        if hub.name.is_none() {
            continue;
        }
        if hub.active {
            if hub.error.is_some() {
                return;
            }
            match hub.state {
                HubState::Disconnected => {
                    prepared = false;
                    command_events.write(HubCommandEvent {
                        hub_id: hub.id,
                        command: HubCommand::Connect,
                    });
                }
                HubState::Connected => {
                    prepared = false;
                    if hub.downloaded {
                        command_events.write(HubCommandEvent {
                            hub_id: hub.id,
                            command: HubCommand::StartProgram,
                        });
                    } else {
                        command_events.write(HubCommandEvent {
                            hub_id: hub.id,
                            command: HubCommand::DownloadProgram,
                        });
                    }
                }
                HubState::Running => {
                    prepared = false;
                    command_events.write(HubCommandEvent {
                        hub_id: hub.id,
                        command: HubCommand::Configure,
                    });
                }
                HubState::Ready => {}
                _ => {
                    prepared = false;
                }
            }

            if !prepared {
                // don't parallelize ble stuff, because downloading is slow otherwise
                // this only makes sense if the query iteration order is deterministic, which it honestly might not be i dunno
                return;
            }
        }
    }
    if prepared {
        println!("Hubs prepared");
        editor_state.set(EditorState::DeviceControl);
    }
}

// runs on enter prepare_control state
fn update_active_hubs(
    mut hubs: Query<&mut BLEHub>,
    q_ble_trains: Query<&BLETrain>,
    q_switch_motors: Query<(&PulseMotor, &LayoutDevice)>,
    q_switches: Query<&Switch>,
    entity_map: Res<EntityMap>,
) {
    let mut active_hub_ids = Vec::new();
    for ble_train in q_ble_trains.iter() {
        if let Some(hub_id) = ble_train.master_hub.hub_id.clone() {
            active_hub_ids.push(hub_id);
        }
        for hub_id in ble_train.iter_puppets().cloned() {
            active_hub_ids.push(hub_id);
        }
    }

    for switch in q_switches.iter() {
        for motor_id_option in switch.motors.iter() {
            if let Some(motor_id) = motor_id_option {
                let entity = entity_map.layout_devices.get(motor_id).unwrap();
                if let Ok((_motor, device)) = q_switch_motors.get(*entity) {
                    if let Some(hub_id) = device.hub_id {
                        active_hub_ids.push(hub_id);
                    }
                }
            }
        }
    }
    for mut hub in hubs.iter_mut() {
        hub.active = active_hub_ids.contains(&hub.id);
    }
}

fn get_hub_configs(
    q_switch_motors: Query<(&PulseMotor, &LayoutDevice)>,
    q_ble_trains: Query<&BLETrain>,
    mut q_hubs: Query<&mut BLEHub>,
    entity_map: Res<EntityMap>,
) {
    for mut hub in q_hubs.iter_mut() {
        hub.config = HubConfiguration::default();
    }
    for (motor, device) in q_switch_motors.iter() {
        for (id, config) in motor.hub_configuration(device) {
            let entity = entity_map.hubs[&id];
            let mut hub = q_hubs.get_mut(entity).unwrap();
            hub.config.merge(&config);
        }
    }
    for ble_train in q_ble_trains.iter() {
        for (id, config) in ble_train.hubs_configuration() {
            let entity = entity_map.hubs[&id];
            let mut hub = q_hubs.get_mut(entity).unwrap();
            hub.config.merge(&config);
        }
    }
}

fn monitor_hub_ready(q_hubs: Query<&BLEHub>, mut editor_state: ResMut<NextState<EditorState>>) {
    for hub in q_hubs.iter() {
        if hub.active {
            match hub.state {
                HubState::Ready => {}
                _ => {
                    warn!("Hub {:?} not ready", hub.id);
                    editor_state.set(EditorState::VirtualControl);
                }
            }
        }
    }
}

fn stop_hub_programs(q_hubs: Query<&BLEHub>, mut command_events: EventWriter<HubCommandEvent>) {
    for hub in q_hubs.iter() {
        if hub.active {
            if hub.state.is_running() {
                command_events.write(HubCommandEvent {
                    hub_id: hub.id,
                    command: HubCommand::StopProgram,
                });
            }
        }
    }
}

pub fn disconnect_hubs(
    q_hubs: Query<&BLEHub>,
    mut command_events: EventWriter<HubCommandEvent>,
    mut next_state: ResMut<NextState<EditorState>>,
) {
    let mut done = true;
    for hub in q_hubs.iter() {
        if hub.state.is_connected() {
            done = false;
            if !hub.state.is_busy() {
                if hub.state.is_running() {
                    command_events.write(HubCommandEvent {
                        hub_id: hub.id,
                        command: HubCommand::StopProgram,
                    });
                } else {
                    command_events.write(HubCommandEvent {
                        hub_id: hub.id,
                        command: HubCommand::Disconnect,
                    });
                }
            }
        }
    }
    if done {
        next_state.set(EditorState::Edit);
    }
}

pub struct BLEPlugin;

impl Plugin for BLEPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SelectablePlugin::<BLEHub>::new());
        app.add_plugins(InspectorPlugin::<BLEHub>::new());
        app.add_event::<HubEvent>();
        app.add_event::<HubCommandEvent>();
        app.add_systems(
            Update,
            (
                spawn_hub.run_if(on_event::<SpawnHubEvent>),
                despawn_hub.run_if(on_event::<DespawnEvent<BLEHub>>),
                delete_selection_shortcut::<BLEHub>,
                handle_hub_events.run_if(on_event::<HubEvent>),
                execute_hub_commands.run_if(on_event::<HubCommandEvent>),
                create_hub,
                prepare_hubs.run_if(in_state(EditorState::PreparingDeviceControl)),
                monitor_hub_ready.run_if(in_state(EditorState::DeviceControl)),
                disconnect_hubs.run_if(in_state(EditorState::Disconnecting)),
            ),
        );
        app.add_systems(
            OnEnter(EditorState::PreparingDeviceControl),
            get_hub_configs,
        );
        app.add_systems(
            OnEnter(EditorState::PreparingDeviceControl),
            update_active_hubs,
        );
        app.add_systems(OnExit(EditorState::DeviceControl), stop_hub_programs);
    }
}
