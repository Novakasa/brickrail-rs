use std::sync::Arc;

use crate::{
    bevy_tokio_tasks::TokioTasksRuntime,
    editor::{GenericID, Selectable, SerializedHub, SpawnEvent},
    layout::EntityMap,
    layout_primitives::{HubID, HubType},
};
use bevy::{input::keyboard, prelude::*};
use bevy_egui::egui::Ui;
use bevy_trait_query::RegisterExt;
use pybricks_ble::{
    io_hub::{IOEvent, IOHub, IOMessage, Input as IOInput},
    pybricks_hub::BLEAdapter,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Resource)]
struct BLEState {
    adapter: Option<Arc<BLEAdapter>>,
}

impl BLEState {}

#[derive(Component)]
struct HubEventReceiver {
    hubs: Vec<HubID>,
    events: Vec<IOMessage>,
}

#[derive(Clone, Default)]
enum HubState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Downloading,
    Ready,
}

#[derive(Component, Serialize, Deserialize, Clone)]
pub struct BLEHub {
    id: HubID,
    #[serde(skip)]
    hub: Arc<Mutex<IOHub>>,
    name: Option<String>,
    #[serde(skip)]
    active: bool,
    #[serde(skip)]
    state: HubState,
}

impl BLEHub {
    pub fn new(id: HubID) -> Self {
        Self {
            id,
            hub: Arc::new(Mutex::new(IOHub::new())),
            name: None,
            active: false,
            state: HubState::Disconnected,
        }
    }
}

impl Selectable for BLEHub {
    fn get_id(&self) -> GenericID {
        GenericID::Hub(self.id)
    }

    fn inspector_ui(&mut self, ui: &mut Ui, context: &mut crate::inspector::InspectorContext) {
        ui.label(format!("BLE Hub {:?}", self.id));
        ui.label(format!(
            "Name: {}",
            self.name.as_deref().unwrap_or("Unknown")
        ));
        if ui
            .button("Discover Name")
            .on_hover_text("Discover the name of the hub")
            .clicked()
        {
            let id = self.id.clone();
            context.commands.add(move |world: &mut World| {
                world.send_event(HubCommandEvent {
                    hub_id: id,
                    command: HubCommand::DiscoverName,
                });
            });
        }
        if self.name.is_some() {
            if ui
                .button("Connect")
                .on_hover_text("Connect to the hub")
                .clicked()
            {
                let id = self.id.clone();
                context.commands.add(move |world: &mut World| {
                    world.send_event(HubCommandEvent {
                        hub_id: id,
                        command: HubCommand::Connect,
                    });
                });
            }
        }
    }
}

fn create_hub(
    mut hub_event_writer: EventWriter<SpawnEvent<SerializedHub>>,
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::H) {
        let id = entity_map.new_hub_id(HubType::Layout);
        let hub = BLEHub::new(id);
        hub_event_writer.send(SpawnEvent(SerializedHub { hub }));
    }
}

fn spawn_hub(
    runtime: Res<TokioTasksRuntime>,
    mut spawn_event_reader: EventReader<SpawnEvent<SerializedHub>>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in spawn_event_reader.read() {
        let hub = event.0.hub.clone();
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
    q_hubs: Query<&BLEHub>,
    entity_map: Res<EntityMap>,
    runtime: Res<TokioTasksRuntime>,
) {
    for event in hub_command_reader.read() {
        let entity = entity_map.hubs[&event.hub_id];
        let hub = q_hubs.get(entity).unwrap();
        match event.command.clone() {
            HubCommand::DiscoverName => {
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.discover_name().await.unwrap();
                });
            }
            HubCommand::Connect => {
                let io_hub = hub.hub.clone();
                let name = hub.name.as_ref().unwrap().clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.connect(&name).await.unwrap();
                });
            }
            HubCommand::QueueInput(input) => {
                let io_hub = hub.hub.clone();
                runtime.spawn_background_task(move |_| async move {
                    io_hub.lock().await.queue_input(input).unwrap();
                });
            }
            _ => {}
        }
    }
}

fn distribute_hub_events(
    mut hub_event_reader: EventReader<HubEvent>,
    mut q_receivers: Query<&mut HubEventReceiver>,
    mut q_hubs: Query<&mut BLEHub>,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in hub_event_reader.read() {
        println!("Event: {:?}", event);
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
                for mut receiver in q_receivers.iter_mut() {
                    if receiver.hubs.contains(&event.hub_id) {
                        receiver.events.push(msg.clone());
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Event, Debug)]
struct HubEvent {
    hub_id: HubID,
    event: IOEvent,
}

fn ble_startup_system(runtime: Res<TokioTasksRuntime>) {
    println!("Starting BLE");
    runtime.spawn_background_task(|mut ctx| async move {
        println!("BLE task");
        let adapter = BLEAdapter::new().await.unwrap();
        ctx.run_on_main_thread(move |ctx| {
            if let Some(mut ble_state) = ctx.world.get_resource_mut::<BLEState>() {
                ble_state.adapter = Some(Arc::new(adapter));
            }
        })
        .await;
    });
}

pub struct BLEPlugin;

impl Plugin for BLEPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BLEState { adapter: None });
        app.register_component_as::<dyn Selectable, BLEHub>();
        app.add_event::<HubEvent>();
        app.add_event::<HubCommandEvent>();
        app.add_systems(Startup, ble_startup_system);
        app.add_systems(
            Update,
            (
                spawn_hub.run_if(on_event::<SpawnEvent<SerializedHub>>()),
                distribute_hub_events.run_if(on_event::<HubEvent>()),
                execute_hub_commands.run_if(on_event::<HubCommandEvent>()),
                create_hub,
            ),
        );
    }
}
