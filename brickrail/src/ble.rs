use std::{path::Path, sync::Arc};

use crate::{
    bevy_tokio_tasks::TokioTasksRuntime,
    ble_train::{BLETrain, TrainData},
    editor::{
        DespawnMessage, EditorState, GenericID, Selection, SelectionState, SpawnHubMessage,
        delete_selection_shortcut,
    },
    inspector::{Inspectable, InspectorPlugin},
    layout::EntityMap,
    layout_devices::LayoutDevice,
    layout_primitives::{HubID, HubPort, HubType},
    persistent_hub_state::PersistentHubState,
    selectable::{Selectable, SelectablePlugin, SelectableType},
    switch::Switch,
    switch_motor::PulseMotor,
};
use bevy::{ecs::system::SystemState, platform::collections::HashMap};
use bevy::{input::keyboard, prelude::*};
use bevy_inspector_egui::bevy_egui::egui::{self, Grid, Ui, widgets::Button};
use pybricks_ble::io_hub::{IOEvent, IOHub, IOMessage, Input as IOInput, SysCode, mod_checksum};
use pybricks_ble::pybricks_hub::HubStatusFlags;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc::UnboundedSender};

#[derive(Component, Debug)]
pub struct HubActive;

#[derive(Component, Debug)]
pub struct HubDownloaded;

#[derive(Component, Debug)]
pub struct HubConnected;

#[derive(Component, Debug)]
pub struct HubRunningProgram;

#[derive(Component, Debug)]
pub struct HubConfigured;

#[derive(Component, Debug)]
pub struct HubOperating;

#[derive(Component, Debug)]
pub struct HubReady;

#[derive(Component, Debug)]
pub struct HubPrepared;

#[derive(Component, Debug)]
pub struct BroadcasterHub;

#[derive(Component, Debug)]
pub struct ObserverHub {
    keep_connected: bool,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub enum HubBusy {
    Connecting,
    Disconnecting,
    Downloading(f32),
    Starting,
    Stopping,
    Configuring,
    SettingReady,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub enum HubError {
    ConnectError,
    ProgramError,
}

#[derive(Message, Debug)]
pub struct HubDeviceStateMessage {
    pub hub_id: HubID,
    pub state_id: u8,
    pub state: u8,
}

fn handle_device_state_msgs(
    mut device_state_reader: MessageReader<HubDeviceStateMessage>,
    mut hub_command_writer: MessageWriter<HubCommandMessage>,
    q_hubs: Query<&BLEHub, Without<ObserverHub>>,
    entity_map: Res<EntityMap>,
) {
    for state_msg in device_state_reader.read() {
        let hub_entity = entity_map.hubs.get(&state_msg.hub_id).unwrap();
        if !q_hubs.contains(*hub_entity) {
            continue;
        }
        let hub_msg = HubCommandMessage {
            hub_id: state_msg.hub_id.clone(),
            command: HubCommand::QueueInput(IOInput::rpc(
                "set_device_state",
                &[state_msg.state_id, state_msg.state],
            )),
        };
        hub_command_writer.write(hub_msg);
    }
}

fn handle_observer_device_state_msgs(
    mut device_state_reader: MessageReader<HubDeviceStateMessage>,
    mut hub_command_writer: MessageWriter<HubCommandMessage>,
    observer_hubs: Query<&BLEHub, With<ObserverHub>>,
    broadcaster: Option<Single<&BLEHub, With<BroadcasterHub>>>,
    entity_map: Res<EntityMap>,
) {
    for state_msg in device_state_reader.read() {
        let hub_entity = entity_map.hubs.get(&state_msg.hub_id).unwrap();
        if !observer_hubs.contains(*hub_entity) {
            continue;
        }
        let observer_hub = observer_hubs.get(*hub_entity).unwrap();
        let hub_msg = HubCommandMessage {
            hub_id: broadcaster.as_ref().unwrap().id.clone(),
            command: HubCommand::QueueInput(IOInput::broadcast_cmd(&[
                observer_hub.name_id().unwrap(),
                state_msg.state_id,
                state_msg.state,
            ])),
        };
        hub_command_writer.write(hub_msg);
    }
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLEHub {
    pub id: HubID,
    #[serde(skip)]
    hub: Arc<Mutex<IOHub>>,
    #[serde(skip)]
    input_sender: Option<UnboundedSender<IOInput>>,
    pub name: Option<String>,
}

impl BLEHub {
    pub fn new(id: HubID) -> Self {
        Self {
            id,
            hub: Arc::new(Mutex::new(IOHub::new())),
            input_sender: None,
            name: None,
        }
    }

    pub fn name_id(&self) -> Option<u8> {
        // checksum of the ascii bytes of the name
        Some(mod_checksum(self.name.as_ref()?.as_bytes()))
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

    pub fn is_marked_downloaded_in_persistent_cache(
        &self,
        settings: &Res<PersistentHubState>,
    ) -> bool {
        if let Some(name) = self.name.as_ref() {
            let hash = self.get_program_hash();
            match settings.program_hashes.get(name) {
                Some(h) => h == &hash,
                None => false,
            }
        } else {
            false
        }
    }

    pub fn sync_persistent_state_downloaded_program(
        &mut self,
        settings: &mut ResMut<PersistentHubState>,
    ) {
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
    type SpawnMessage = SpawnHubMessage;
    type ID = HubID;

    fn get_type() -> SelectableType {
        SelectableType::Hub
    }

    fn generic_id(&self) -> GenericID {
        GenericID::Hub(self.id)
    }

    fn default_spawn_event(entity_map: &mut ResMut<EntityMap>) -> Option<Self::SpawnMessage> {
        Some(SpawnHubMessage {
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
            Query<(
                &BLEHub,
                Option<&HubBusy>,
                Option<&HubConnected>,
                Option<&HubRunningProgram>,
                Option<&HubDownloaded>,
                Option<&HubPrepared>,
                Option<&mut ObserverHub>,
            )>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
            MessageWriter<HubCommandMessage>,
            Commands,
        )>::new(world);
        let (
            mut hubs,
            entity_map,
            selection_state,
            _type_registry,
            mut command_messages,
            mut commands,
        ) = state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok((hub, busy, connected, running, downloaded, ready, maybe_observer)) =
                hubs.get_mut(entity)
            {
                ui.label(format!("BLE Hub {:?}", hub.id));
                ui.label(format!(
                    "Name: {}",
                    hub.name.as_deref().unwrap_or("Unknown")
                ));
                ui.label(format!("name id: {:?}", hub.name_id()));
                ui.label(format!("{:?}", busy));
                ui.label(format!("{:?}", connected));
                ui.label(format!("{:?}", running));
                ui.label(format!("{:?}", downloaded));
                ui.label(format!("{:?}", ready));
                if ui
                    .button("Discover Name")
                    .on_hover_text("Discover the name of the hub")
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::DiscoverName,
                    });
                }
                if ui
                    .add_enabled(
                        hub.name.is_some() && connected.is_none() && busy.is_none(),
                        Button::new("Connect"),
                    )
                    .on_hover_text("Connect to the hub")
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::Connect,
                    });
                }
                if ui
                    .add_enabled(
                        connected.is_some() && busy.is_none(),
                        Button::new("Disconnect"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::Disconnect,
                    });
                }
                if ui
                    .add_enabled(
                        connected.is_some() && busy.is_none(),
                        Button::new("Download Program"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::DownloadProgram,
                    });
                }
                if ui
                    .add_enabled(
                        downloaded.is_some()
                            && busy.is_none()
                            && connected.is_some()
                            && running.is_none(),
                        Button::new("Start Program"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::StartProgram,
                    });
                }
                if ui
                    .add_enabled(
                        connected.is_some() && busy.is_none() && running.is_some(),
                        Button::new("Stop Program"),
                    )
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_messages.write(HubCommandMessage {
                        hub_id: id,
                        command: HubCommand::StopProgram,
                    });
                }
                ui.separator();
                let mut is_observer = maybe_observer.is_some();
                if ui.checkbox(&mut is_observer, "Observer Hub").changed() {
                    let entity = entity_map.hubs[&hub.id];
                    if is_observer {
                        commands.entity(entity).insert(ObserverHub {
                            keep_connected: false,
                        });
                    } else {
                        commands.entity(entity).remove::<ObserverHub>();
                    }
                }
                if let Some(mut observer) = maybe_observer {
                    ui.checkbox(&mut observer.keep_connected, "Keep Connected");
                }
            }
        }
        state.apply(world);
    }

    pub fn select_port_ui(
        ui: &mut Ui,
        selected_hub: &mut Option<HubID>,
        selected_port: &mut Option<HubPort>,
        kind: HubType,
        hubs: &Query<&BLEHub>,
        spawn_messages: &mut MessageWriter<SpawnHubMessage>,
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
                    spawn_messages,
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
        spawn_messages: &mut MessageWriter<SpawnHubMessage>,
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
                        spawn_messages.write(SpawnHubMessage { hub });
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
    mut hub_message_writer: MessageWriter<SpawnHubMessage>,
    keyboard_input: Res<ButtonInput<keyboard::KeyCode>>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::KeyH) {
        let id = entity_map.new_hub_id(HubType::Layout);
        let hub = BLEHub::new(id);
        hub_message_writer.write(SpawnHubMessage { hub });
    }
}

fn spawn_hub(
    runtime: Res<TokioTasksRuntime>,
    mut spawn_event_reader: MessageReader<SpawnHubMessage>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
    settings: Res<PersistentHubState>,
) {
    for event in spawn_event_reader.read() {
        let hub = event.hub.clone();
        println!("name: {:?}", hub.name);
        let hub_id = hub.id;
        let hub_mutex = hub.hub.clone();
        let name = Name::new(hub.name.clone().unwrap_or(hub_id.to_string()));
        let is_marked_downloaded_in_settings =
            hub.is_marked_downloaded_in_persistent_cache(&settings);
        let entity = commands.spawn((name, hub)).id();

        if is_marked_downloaded_in_settings {
            commands.entity(entity).insert(HubDownloaded);
        }
        entity_map.add_hub(hub_id, entity);

        runtime.spawn_background_task(move |mut ctx| async move {
            let mut event_receiver = hub_mutex.lock().await.subscribe_events();
            println!("Listening for messages on hub {:?}", hub_id);
            while let Ok(event) = event_receiver.recv().await {
                ctx.run_on_main_thread(move |ctx| {
                    ctx.world.write_message(HubMessage {
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
    mut hub_event_reader: MessageReader<DespawnMessage<BLEHub>>,
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

#[derive(Component, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HubConfiguration {
    data: HashMap<u8, u32>,
}

impl HubConfiguration {
    pub fn add_value(&mut self, address: u8, value: u32) {
        self.data.insert(address, value);
    }

    pub fn merge(&mut self, other: &Self) {
        for (address, value) in other.data.iter() {
            assert!(
                !self.data.contains_key(address),
                "Address {} already exists in HubConfiguration, cannot merge",
                address
            );
            self.data.insert(*address, *value);
        }
    }
}

#[derive(Debug, Clone)]
pub enum HubCommType {
    Observer,
    Broadcaster,
    Regular,
}

impl HubCommType {
    fn from_query(
        maybe_observer: Option<&ObserverHub>,
        maybe_broadcaster: Option<&BroadcasterHub>,
    ) -> Self {
        match (maybe_observer.is_some(), maybe_broadcaster.is_some()) {
            (true, false) => HubCommType::Observer,
            (false, true) => HubCommType::Broadcaster,
            (false, false) => HubCommType::Regular,
            (true, true) => panic!("Hub cannot be both observer and broadcaster"),
        }
    }

    fn to_u8(&self) -> u8 {
        match self {
            HubCommType::Observer => 0x01,
            HubCommType::Broadcaster => 0x02,
            HubCommType::Regular => 0x00,
        }
    }
}

#[derive(Message, Debug, Clone)]
pub enum HubCommand {
    DiscoverName,
    Connect,
    Disconnect,
    DownloadProgram,
    StartProgram,
    StopProgram,
    QueueInput(IOInput),
    Configure,
    SetReady,
}

#[derive(Message, Debug)]
pub struct HubCommandMessage {
    pub hub_id: HubID,
    pub command: HubCommand,
}

impl HubCommandMessage {
    pub fn input(hub_id: HubID, input: IOInput) -> Self {
        Self {
            hub_id,
            command: HubCommand::QueueInput(input),
        }
    }
}

fn execute_hub_commands(
    mut hub_command_reader: MessageReader<HubCommandMessage>,
    q_hubs: Query<(&BLEHub, Option<&HubConfiguration>)>,
    entity_map: Res<EntityMap>,
    runtime: Res<TokioTasksRuntime>,
    mut commands: Commands,
    mut persistent_hub_state: ResMut<PersistentHubState>,
) {
    for event in hub_command_reader.read() {
        let entity = entity_map.hubs[&event.hub_id];
        let (hub, maybe_config) = q_hubs.get(entity).unwrap();
        match event.command.clone() {
            HubCommand::DiscoverName => {
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.discover_name().await.unwrap();
                });
            }
            HubCommand::Connect => {
                commands.entity(entity).insert(HubBusy::Connecting);
                let io_hub = hub.hub.clone();
                let name = hub.name.as_ref().unwrap().clone();
                runtime.spawn_background_task(move |mut ctx| async move {
                    if io_hub.lock().await.connect(&name).await.is_err() {
                        ctx.run_on_main_thread(move |ctx_main| {
                            let mut system_state: SystemState<Commands> =
                                SystemState::new(ctx_main.world);
                            let mut commands = system_state.get_mut(ctx_main.world);
                            commands
                                .entity(entity)
                                .insert(HubError::ConnectError)
                                .remove::<HubBusy>();
                            system_state.apply(ctx_main.world);
                        })
                        .await;
                    }
                });
            }
            HubCommand::Disconnect => {
                commands
                    .entity(entity)
                    .insert(HubBusy::Disconnecting)
                    .remove::<HubConnected>();
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |mut ctx| async move {
                    io_hub.lock().await.disconnect().await.unwrap();
                    info!("Disconnected hub");
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<Commands> =
                            SystemState::new(ctx_main.world);
                        let mut commands = system_state.get_mut(ctx_main.world);
                        commands.entity(entity).remove::<HubBusy>();
                        system_state.apply(ctx_main.world);
                    })
                    .await;
                });
            }
            HubCommand::DownloadProgram => {
                commands.entity(entity).insert(HubBusy::Downloading(0.0));
                let io_hub = hub.hub.clone();
                let program = hub.get_program_path();
                runtime.spawn_background_task(move |mut ctx| async move {
                    io_hub.lock().await.download_program(program).await.unwrap();
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<(
                            Query<&mut BLEHub>,
                            ResMut<PersistentHubState>,
                            Commands,
                        )> = SystemState::new(ctx_main.world);
                        let (mut query, mut persistent_hub_state, mut commands) =
                            system_state.get_mut(ctx_main.world);
                        let mut hub = query.get_mut(entity).unwrap();

                        commands
                            .entity(entity)
                            .remove::<HubBusy>()
                            .insert(HubDownloaded);
                        hub.sync_persistent_state_downloaded_program(&mut persistent_hub_state);
                        system_state.apply(ctx_main.world);
                    })
                    .await;
                });
            }
            HubCommand::StartProgram => {
                commands.entity(entity).insert(HubBusy::Starting);
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
                        system_state.apply(ctx_main.world);
                    })
                    .await;
                });
            }
            HubCommand::StopProgram => {
                commands.entity(entity).insert(HubBusy::Stopping);
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.stop_program().await.unwrap();
                });
            }
            HubCommand::QueueInput(input) => {
                hub.input_sender.as_ref().unwrap().send(input).unwrap();
            }
            HubCommand::Configure => {
                commands.entity(entity).insert(HubBusy::Configuring);
                let sender = hub.input_sender.as_ref().unwrap();
                for (address, value) in maybe_config.unwrap().data.iter() {
                    sender.send(IOInput::store_uint(*address, *value)).unwrap();
                }
                persistent_hub_state
                    .sync_configured_hub(hub.name.as_ref().unwrap(), maybe_config.unwrap());
                commands
                    .entity(entity)
                    .remove::<HubBusy>()
                    .insert(HubConfigured);
            }
            HubCommand::SetReady => {
                commands.entity(entity).insert(HubBusy::SettingReady);
                let sender = hub.input_sender.as_ref().unwrap();
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

#[derive(Message, Debug)]
pub struct HubMessageMessage<T: FromIOMessage> {
    pub id: HubID,
    pub data: T,
}

fn handle_hub_messages(
    mut hub_message_reader: MessageReader<HubMessage>,
    mut train_sender: MessageWriter<HubMessageMessage<TrainData>>,
    mut q_hubs: Query<(
        &mut BLEHub,
        &mut Name,
        Option<&HubBusy>,
        Option<&HubRunningProgram>,
        Option<&HubConnected>,
    )>,
    entity_map: Res<EntityMap>,
    settings: Res<PersistentHubState>,
    mut commands: Commands,
) {
    for event in hub_message_reader.read() {
        let entity = entity_map.hubs[&event.hub_id];
        let (mut hub, mut name_component, maybe_hub_busy, maybe_hub_running, maybe_connected) =
            q_hubs.get_mut(entity).unwrap();
        match &event.event {
            IOEvent::NameDiscovered(name) => {
                hub.name = Some(name.clone());
                name_component.set(name.clone());
                hub.is_marked_downloaded_in_persistent_cache(&settings);
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
                                commands.entity(entity).insert(HubReady);
                                if maybe_hub_busy == Some(&HubBusy::SettingReady) {
                                    commands.entity(entity).remove::<HubBusy>();
                                } else {
                                    warn!("Hub reported ready, but was not setting ready");
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => match hub.id.kind {
                        HubType::Train => {
                            if let Some(data) = TrainData::from_io_message(msg) {
                                debug!("sending TrainData: {:?}", data);
                                train_sender.write(HubMessageMessage { id: hub.id, data });
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
                if maybe_hub_busy == Some(&HubBusy::Connecting) {
                    if maybe_connected.is_some() {
                        error!("Was in connecting state but already connected");
                    }
                    commands.entity(entity).remove::<HubBusy>();
                    commands.entity(entity).insert(HubConnected);
                }
                let running_flag = status.flags.clone() & HubStatusFlags::PROGRAM_RUNNING
                    == HubStatusFlags::PROGRAM_RUNNING;
                if running_flag && maybe_hub_running.is_none() {
                    match maybe_hub_busy {
                        Some(HubBusy::Starting) => {
                            commands
                                .entity(entity)
                                .remove::<HubBusy>()
                                .insert(HubRunningProgram);
                        }
                        _ => {
                            warn!("Hub reported running program, but was not starting");
                            commands.entity(entity).insert(HubRunningProgram); // to make sure prepare_hubs doesn't try to configure yet
                        }
                    }
                }
                if !running_flag && maybe_hub_running.is_some() {
                    commands.entity(entity).remove::<HubRunningProgram>();
                    if let Some(HubBusy::Stopping) = maybe_hub_busy {
                        commands.entity(entity).remove::<HubBusy>();
                    } else {
                        commands.entity(entity).insert(HubError::ProgramError);
                        if let Some(HubBusy::Configuring) = maybe_hub_busy {
                            warn!("Hub reported program stopped while configuring");
                            commands.entity(entity).remove::<HubBusy>();
                        } else {
                            warn!("Hub reported stopped program, but was not stopping");
                        }
                    }
                }
            }
            IOEvent::DownloadProgress(progress) => {
                // info!("Download progress: {:?}", progress);
                commands
                    .entity(entity)
                    .insert(HubBusy::Downloading(*progress));
            }
        }
    }
}

#[derive(Message, Debug)]
struct HubMessage {
    hub_id: HubID,
    event: IOEvent,
}

pub fn prepare_hubs(
    q_hubs_not_busy: Query<
        (
            &BLEHub,
            Option<&HubConnected>,
            Option<&HubDownloaded>,
            Option<&HubRunningProgram>,
            Option<&ObserverHub>,
            Option<&HubConfigured>,
            Option<&HubReady>,
        ),
        (
            Without<HubError>,
            With<HubActive>,
            Without<HubBusy>,
            Without<HubPrepared>,
        ),
    >,
    q_hubs_busy: Query<&HubBusy>,

    mut command_messages: MessageWriter<HubCommandMessage>,
) {
    if !q_hubs_busy.is_empty() {
        return;
    }
    for (
        hub,
        maybe_connected,
        maybe_downloaded,
        maybe_running,
        maybe_observer,
        maybe_configured,
        maybe_ready,
    ) in q_hubs_not_busy.iter()
    {
        if hub.name.is_none() {
            error!("Hub {:?} has no name, cannot prepare", hub.id);
            continue;
        }

        if maybe_connected.is_none() && maybe_running.is_none() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::Connect,
            });
            return;
        }
        if maybe_downloaded.is_none() && maybe_running.is_none() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::DownloadProgram,
            });
            return;
        }
        if maybe_running.is_none() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::StartProgram,
            });
            return;
        }
        if maybe_configured.is_none() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::Configure,
            });
            return;
        }
        if maybe_ready.is_none() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::SetReady,
            });
            return;
        }
        if let Some(observer) = maybe_observer {
            if !observer.keep_connected {
                info!("Observer hub disconnecting...");
                command_messages.write(HubCommandMessage {
                    hub_id: hub.id,
                    command: HubCommand::Disconnect,
                });
                return;
            }
        }
    }
}

fn finalize_hub_preparation(
    q_hubs: Query<&BLEHub, (With<HubActive>, Without<HubPrepared>)>,
    mut editor_state: ResMut<NextState<EditorState>>,
) {
    if q_hubs.is_empty() {
        // all hubs are ready
        println!("Hubs prepared");
        editor_state.set(EditorState::DeviceControl);
    }
}

// runs on enter prepare_control state
fn update_active_hubs(
    hubs: Query<(Entity, &BLEHub)>,
    q_ble_trains: Query<&BLETrain>,
    q_switch_motors: Query<(&PulseMotor, &LayoutDevice)>,
    q_switches: Query<&Switch>,
    entity_map: Res<EntityMap>,
    mut commands: Commands,
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
    for (entity, hub) in hubs.iter() {
        if active_hub_ids.contains(&hub.id) {
            commands.entity(entity).insert(HubActive);
        } else {
            commands.entity(entity).remove::<HubActive>();
        }
    }
}

fn get_hub_configs(
    q_switch_motors: Query<(&PulseMotor, &LayoutDevice)>,
    q_ble_trains: Query<&BLETrain>,
    q_hubs: Query<(
        Entity,
        &BLEHub,
        Option<&ObserverHub>,
        Option<&BroadcasterHub>,
    )>,
    mut commands: Commands,
) {
    let mut configs = HashMap::new();
    for (_entity, hub, maybe_observer, maybe_broadcaster) in q_hubs.iter() {
        let mut config = HubConfiguration::default();
        config.add_value(
            30,
            HubCommType::from_query(maybe_observer, maybe_broadcaster).to_u8() as u32,
        );
        configs.insert(hub.id, config);
    }
    for (motor, device) in q_switch_motors.iter() {
        for (id, config) in motor.hub_configuration(device) {
            configs.get_mut(&id).unwrap().merge(&config);
        }
    }
    for ble_train in q_ble_trains.iter() {
        for (id, config) in ble_train.hubs_configuration() {
            configs.get_mut(&id).unwrap().merge(&config);
        }
    }
    for (entity, hub, _, _) in q_hubs.iter() {
        commands
            .entity(entity)
            .insert(configs.remove(&hub.id).unwrap());
    }
}

fn check_hub_prepared(
    q_hubs: Query<
        (
            Entity,
            &BLEHub,
            Option<&HubConnected>,
            Option<&HubBusy>,
            Option<&HubRunningProgram>,
            Option<&ObserverHub>,
            Option<&HubConfigured>,
            Option<&HubPrepared>,
            Option<&HubReady>,
        ),
        With<HubActive>,
    >,
    mut commands: Commands,
) {
    for (
        entity,
        hub,
        maybe_connected,
        maybe_busy,
        maybe_running,
        maybe_observer,
        maybe_configured,
        maybe_prepared,
        maybe_ready,
    ) in q_hubs.iter()
    {
        // println!(
        //     "Checking hub {:?}: connected={:?}, busy={:?}, running={:?}",
        //     hub.id, maybe_connected, maybe_busy, maybe_running
        // );
        if let Some(observer) = maybe_observer {
            if maybe_prepared.is_some() {
                if maybe_running.is_none()
                    || maybe_configured.is_none()
                    || (maybe_connected.is_none() && observer.keep_connected)
                    || maybe_ready.is_none()
                {
                    warn!(
                        "Observer hub {:?} no longer ready",
                        hub.name.as_ref().unwrap()
                    );
                    commands
                        .entity(entity)
                        .remove::<HubPrepared>()
                        .remove::<HubConfigured>()
                        .remove::<HubRunningProgram>() // forces us to re-start, which makes sense if we want to reconfigure
                        .remove::<HubReady>();
                }
            } else {
                // println!(
                //     "Observer hub {:?} checking ready: running={:?}, configured={:?}, connected={:?}, busy={:?}",
                //     hub.name.as_ref().unwrap(),
                //     maybe_running,
                //     maybe_configured,
                //     maybe_connected,
                //     maybe_busy,
                // );
                if maybe_running.is_some()
                    && maybe_configured.is_some()
                    && (maybe_connected.is_none() || observer.keep_connected)
                    && maybe_busy.is_none()
                    && maybe_ready.is_some()
                {
                    info!("Observer hub {:?} is ready", hub.name.as_ref().unwrap());
                    commands.entity(entity).insert(HubPrepared);
                }
            }
        } else {
            // regular or broadcaster hub
            if maybe_prepared.is_some() {
                if maybe_busy.is_some()
                    || maybe_running.is_none()
                    || maybe_configured.is_none()
                    || maybe_connected.is_none()
                    || maybe_ready.is_none()
                {
                    warn!("Hub {:?} no longer prepared", hub.name.as_ref().unwrap());
                    commands
                        .entity(entity)
                        .remove::<HubPrepared>()
                        .remove::<HubConfigured>()
                        .remove::<HubReady>();
                }
            } else {
                if maybe_connected.is_some()
                    && maybe_busy.is_none()
                    && maybe_running.is_some()
                    && maybe_configured.is_some()
                    && maybe_ready.is_some()
                {
                    info!("Hub {:?} is prepared", hub.name.as_ref().unwrap());
                    commands.entity(entity).insert(HubPrepared);
                }
            }
        }
    }
}

fn monitor_non_prepared_hubs(
    q_hubs: Query<&BLEHub, (With<HubActive>, Without<HubPrepared>)>,
    mut editor_state: ResMut<NextState<EditorState>>,
) {
    for hub in q_hubs.iter() {
        warn!("Hub {:?} not ready", hub.id);
        editor_state.set(EditorState::VirtualControl);
        return;
    }
}

fn stop_hub_programs(
    q_hubs: Query<
        &BLEHub,
        (
            With<HubRunningProgram>,
            With<HubActive>,
            Without<ObserverHub>,
        ),
    >,
    mut command_messages: MessageWriter<HubCommandMessage>,
) {
    info!("Stopping hub programs, because exiting Device Control mode");
    for hub in q_hubs.iter() {
        command_messages.write(HubCommandMessage {
            hub_id: hub.id,
            command: HubCommand::StopProgram,
        });
    }
}

pub fn disconnect_hubs(
    q_hubs: Query<(&BLEHub, Option<&HubRunningProgram>), (With<HubConnected>, Without<HubBusy>)>,
    q_hubs_busy: Query<&BLEHub, With<HubBusy>>,
    mut command_messages: MessageWriter<HubCommandMessage>,
) {
    if !q_hubs_busy.is_empty() {
        // println!("Waiting for busy hubs to finish before disconnecting");
        return;
    }
    for (hub, running_program) in q_hubs.iter() {
        if running_program.is_some() {
            command_messages.write(HubCommandMessage {
                hub_id: hub.id,
                command: HubCommand::StopProgram,
            });
            return;
        }
        command_messages.write(HubCommandMessage {
            hub_id: hub.id,
            command: HubCommand::Disconnect,
        });
        return;
    }
}

pub fn finalize_disconnection(
    busy_hubs: Query<&BLEHub, With<HubBusy>>,
    q_hubs: Query<&BLEHub, (With<HubConnected>, Without<HubBusy>)>,
    mut next_state: ResMut<NextState<EditorState>>,
) {
    if !busy_hubs.is_empty() {
        // println!("Waiting for busy hubs to finish before finalizing disconnection");
        return;
    }
    if q_hubs.is_empty() {
        println!("All hubs disconnected");
        next_state.set(EditorState::VirtualControl);
    }
}

pub fn ensure_broadcaster_hub(
    mut commands: Commands,
    normal_hubs: Query<(Entity, &BLEHub), (Without<BroadcasterHub>, Without<ObserverHub>)>,
    broadcaster_hub: Option<Single<&BroadcasterHub, With<HubActive>>>,
) {
    if broadcaster_hub.is_none() {
        if let Some((entity, hub)) = normal_hubs.iter().next() {
            info!(
                "Marking hub as broadcaster hub {:?}",
                hub.name.as_ref().unwrap()
            );
            commands.entity(entity).insert(BroadcasterHub);
        }
    }
}

pub struct BLEPlugin;

impl Plugin for BLEPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SelectablePlugin::<BLEHub>::new());
        app.add_plugins(InspectorPlugin::<BLEHub>::new());
        app.add_message::<HubMessage>();
        app.add_message::<HubCommandMessage>();
        app.add_message::<HubDeviceStateMessage>();
        app.add_systems(
            Update,
            (
                spawn_hub.run_if(on_message::<SpawnHubMessage>),
                despawn_hub.run_if(on_message::<DespawnMessage<BLEHub>>),
                delete_selection_shortcut::<BLEHub>,
                create_hub,
                (
                    handle_device_state_msgs.run_if(on_message::<HubDeviceStateMessage>),
                    handle_observer_device_state_msgs.run_if(on_message::<HubDeviceStateMessage>),
                    handle_hub_messages.run_if(on_message::<HubMessage>),
                    monitor_non_prepared_hubs.run_if(in_state(EditorState::DeviceControl)),
                    finalize_hub_preparation.run_if(in_state(EditorState::PreparingDeviceControl)),
                    disconnect_hubs.run_if(in_state(EditorState::Disconnecting)),
                    finalize_disconnection.run_if(in_state(EditorState::Disconnecting)),
                    check_hub_prepared,
                    prepare_hubs.run_if(in_state(EditorState::PreparingDeviceControl)),
                    execute_hub_commands.run_if(on_message::<HubCommandMessage>),
                )
                    .chain(),
            ),
        );
        app.add_systems(
            OnEnter(EditorState::PreparingDeviceControl),
            (
                get_hub_configs.after(ensure_broadcaster_hub),
                update_active_hubs,
                ensure_broadcaster_hub,
            ),
        );
        app.add_systems(OnExit(EditorState::DeviceControl), stop_hub_programs);
    }
}
