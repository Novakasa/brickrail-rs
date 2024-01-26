use std::sync::Arc;

use crate::{
    bevy_tokio_tasks::TokioTasksRuntime,
    editor::{GenericID, Selectable, SpawnEvent},
    layout::EntityMap,
    layout_primitives::HubID,
};
use bevy::{input::keyboard, prelude::*};
use pybricks_ble::{
    io_hub::{IOEvent, IOHub},
    pybricks_hub::BLEAdapter,
};
use serde::{Deserialize, Serialize};

#[derive(Resource)]
struct BLEState {
    adapter: Option<Arc<BLEAdapter>>,
}

impl BLEState {}

#[derive(Component)]
struct HubEventReceiver {
    hubs: Vec<HubID>,
    events: Vec<IOEvent>,
}

#[derive(Component, Serialize, Deserialize, Clone)]
struct BLEHub {
    id: HubID,
    #[serde(skip)]
    hub: Arc<IOHub>,
    name: Option<String>,
}

impl Selectable for BLEHub {
    fn get_id(&self) -> GenericID {
        GenericID::Hub(self.id)
    }
}

fn discover_hub_name(
    q_hubs: Query<&BLEHub>,
    runtime: Res<TokioTasksRuntime>,
    keyboard_input: Res<Input<keyboard::KeyCode>>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::D) {
        for hub in q_hubs.iter() {
            let io_hub = hub.hub.clone();
            if hub.name.is_none() {
                runtime.spawn_background_task(move |_| async move {
                    io_hub.discover_name().await.unwrap();
                });
                return;
            }
        }
    }
}

fn create_hub(
    mut hub_event_writer: EventWriter<SpawnEvent<BLEHub>>,
    keyboard_input: Res<Input<keyboard::KeyCode>>,
    entity_map: Res<EntityMap>,
) {
    if keyboard_input.just_pressed(keyboard::KeyCode::H) {
        let id = entity_map.new_hub_id();
        let hub = BLEHub {
            id,
            hub: Arc::new(IOHub::new()),
            name: None,
        };
        hub_event_writer.send(SpawnEvent(hub));
    }
}

fn spawn_hub(
    runtime: Res<TokioTasksRuntime>,
    mut spawn_event_reader: EventReader<SpawnEvent<BLEHub>>,
    mut commands: Commands,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in spawn_event_reader.read() {
        let hub = event.0.clone();
        let hub_id = hub.id;
        let mut event_receiver = hub.hub.subscribe_events();
        let entity = commands.spawn(hub).id();
        entity_map.add_hub(hub_id, entity);
        runtime.spawn_background_task(move |mut ctx| async move {
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

fn distribute_hub_events(
    mut hub_event_reader: EventReader<HubEvent>,
    mut q_receivers: Query<&mut HubEventReceiver>,
    mut q_hubs: Query<&mut BLEHub>,
) {
    for event in hub_event_reader.read() {
        println!("Event: {:?}", event);
        if let IOEvent::NameDiscovered(name) = &event.event {
            for mut hub in q_hubs.iter_mut() {
                if hub.id == event.hub_id {
                    hub.name = Some(name.clone());
                    return;
                }
            }
        }
        for mut receiver in q_receivers.iter_mut() {
            if receiver.hubs.contains(&event.hub_id) {
                receiver.events.push(event.event.clone());
            }
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
        app.add_event::<HubEvent>();
        app.add_event::<SpawnEvent<BLEHub>>();
        app.add_systems(Startup, ble_startup_system);
        app.add_systems(
            Update,
            (
                spawn_hub,
                distribute_hub_events,
                create_hub,
                discover_hub_name,
            ),
        );
    }
}
