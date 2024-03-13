use std::{path::Path, sync::Arc};

use crate::{
    bevy_tokio_tasks::TokioTasksRuntime,
    ble_train::TrainData,
    editor::{EditorState, GenericID, Selectable, Selection, SelectionState, SpawnHubEvent},
    layout::EntityMap,
    layout_primitives::{HubID, HubPort, HubType},
};
use bevy::{input::keyboard, prelude::*};
use bevy_ecs::system::SystemState;
use bevy_egui::egui::{self, widgets::Button, Layout, Ui};
use bevy_trait_query::RegisterExt;
use pybricks_ble::io_hub::{IOEvent, IOHub, IOMessage, Input as IOInput};
use pybricks_ble::pybricks_hub::HubStatus;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub enum HubState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Downloading,
    StartingProgram,
    Running,
    StoppingProgram,
    Disconnecting,
    ProgramError,
    ConnectError,
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLEHub {
    pub id: HubID,
    #[serde(skip)]
    hub: Arc<Mutex<IOHub>>,
    #[serde(skip)]
    input_sender: Option<tokio::sync::mpsc::UnboundedSender<IOInput>>,
    pub name: Option<String>,
    #[serde(skip)]
    pub active: bool,
    #[serde(skip)]
    pub state: HubState,
    #[serde(skip)]
    downloaded: bool,
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
            downloaded: false,
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
}

impl Selectable for BLEHub {
    fn get_id(&self) -> GenericID {
        GenericID::Hub(self.id)
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
                    command_events.send(HubCommandEvent {
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
                    command_events.send(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::Connect,
                    });
                }
                if ui
                    .add_enabled(hub.state == HubState::Connected, Button::new("Disconnect"))
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.send(HubCommandEvent {
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
                    command_events.send(HubCommandEvent {
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
                    command_events.send(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::StartProgram,
                    });
                }
                if ui
                    .add_enabled(hub.state == HubState::Running, Button::new("Stop Program"))
                    .clicked()
                {
                    let id = hub.id.clone();
                    command_events.send(HubCommandEvent {
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
        ui.label("Port");
        ui.add_enabled_ui(selected_hub.is_some(), |ui| {
            ui.push_id("port", |ui| {
                ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                    egui::ComboBox::from_label("")
                        .selected_text(format!(
                            "{:}",
                            selected_port
                                .map(|h| h.to_string())
                                .unwrap_or("None".to_string())
                        ))
                        .show_ui(ui, |ui| {
                            for option in [
                                None,
                                Some(HubPort::A),
                                Some(HubPort::B),
                                Some(HubPort::C),
                                Some(HubPort::D),
                                Some(HubPort::E),
                                Some(HubPort::F),
                            ] {
                                ui.selectable_value(
                                    selected_port,
                                    option,
                                    format!(
                                        "{:}",
                                        option.map(|h| h.to_string()).unwrap_or("None".to_string())
                                    ),
                                );
                            }
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
        ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
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
                        spawn_events.send(SpawnHubEvent { hub });
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
        hub_event_writer.send(SpawnHubEvent { hub });
    }
}

fn spawn_hub(
    runtime: Res<TokioTasksRuntime>,
    mut spawn_event_reader: EventReader<SpawnHubEvent>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in spawn_event_reader.read() {
        let hub = event.hub.clone();
        let hub_id = hub.id;
        if let Some(name) = &hub.name {
            entity_map
                .names
                .insert(GenericID::Hub(hub_id), name.clone());
        }
        let hub_mutex = hub.hub.clone();
        let entity = commands.spawn(hub).id();
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

#[derive(Event, Debug, Clone)]
pub enum HubCommand {
    DiscoverName,
    Connect,
    Disconnect,
    DownloadProgram,
    StartProgram,
    StopProgram,
    QueueInput(IOInput),
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
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.connect(&name).await.unwrap();
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
                hub.state = HubState::Downloading;
                let io_hub = hub.hub.clone();
                let program = hub.get_program_path();
                runtime.spawn_background_task(move |mut ctx| async move {
                    io_hub.lock().await.download_program(program).await.unwrap();
                    ctx.run_on_main_thread(move |ctx_main| {
                        let mut system_state: SystemState<(Query<&mut BLEHub>,)> =
                            SystemState::new(ctx_main.world);
                        let mut query = system_state.get_mut(ctx_main.world);
                        let mut hub = query.0.get_mut(entity).unwrap();
                        hub.downloaded = true;
                        hub.state = HubState::Connected;
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
    mut q_hubs: Query<&mut BLEHub>,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in hub_event_reader.read() {
        let mut hub = q_hubs.get_mut(entity_map.hubs[&event.hub_id]).unwrap();
        match &event.event {
            IOEvent::NameDiscovered(name) => {
                hub.name = Some(name.clone());
                entity_map
                    .names
                    .insert(GenericID::Hub(hub.id), name.clone());
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
                    }
                    _ => match hub.id.kind {
                        HubType::Train => {
                            if let Some(data) = TrainData::from_io_message(msg) {
                                debug!("sending TrainData: {:?}", data);
                                train_sender.send(HubMessageEvent { id: hub.id, data });
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
                let running_flag =
                    status.clone() & HubStatus::PROGRAM_RUNNING == HubStatus::PROGRAM_RUNNING;
                if running_flag {
                    if hub.state == HubState::StartingProgram {
                        hub.state = HubState::Running;
                    }
                } else {
                    match hub.state {
                        HubState::Running => {
                            hub.state = HubState::ProgramError;
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
            match hub.state {
                HubState::Disconnected => {
                    prepared = false;
                    command_events.send(HubCommandEvent {
                        hub_id: hub.id,
                        command: HubCommand::Connect,
                    });
                }
                HubState::Connected => {
                    prepared = false;
                    if hub.downloaded {
                        command_events.send(HubCommandEvent {
                            hub_id: hub.id,
                            command: HubCommand::StartProgram,
                        });
                    } else {
                        command_events.send(HubCommandEvent {
                            hub_id: hub.id,
                            command: HubCommand::DownloadProgram,
                        });
                    }
                }
                HubState::Running => {}
                HubState::ConnectError | HubState::ProgramError => {
                    prepared = false;
                    editor_state.set(EditorState::Edit);
                }
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

fn monitor_hub_ready(q_hubs: Query<&BLEHub>, mut editor_state: ResMut<NextState<EditorState>>) {
    for hub in q_hubs.iter() {
        if hub.active {
            match hub.state {
                HubState::Running => {}
                _ => {
                    warn!("Hub {:?} not ready", hub.id);
                    editor_state.set(EditorState::VirtualControl);
                }
            }
        }
    }
}

pub struct BLEPlugin;

impl Plugin for BLEPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Selectable, BLEHub>();
        app.add_event::<HubEvent>();
        app.add_event::<HubCommandEvent>();
        app.add_systems(
            Update,
            (
                spawn_hub.run_if(on_event::<SpawnHubEvent>()),
                handle_hub_events.run_if(on_event::<HubEvent>()),
                execute_hub_commands.run_if(on_event::<HubCommandEvent>()),
                create_hub,
                prepare_hubs.run_if(in_state(EditorState::PreparingDeviceControl)),
                monitor_hub_ready.run_if(in_state(EditorState::DeviceControl)),
            ),
        );
    }
}
