use crate::bevy_tokio_tasks::TokioTasksRuntime;
use bevy::prelude::*;
use pybricks_ble::{io_hub::IOHub, pybricks_hub::BLEAdapter};
use serde::{Deserialize, Serialize};

#[derive(Resource)]
struct BLEState {
    adapter: Option<BLEAdapter>,
}

impl BLEState {}

#[derive(Component, Serialize, Deserialize)]
struct BLEHub {
    #[serde(skip)]
    hub: IOHub,
    name: Option<String>,
}

fn ble_startup_system(runtime: Res<TokioTasksRuntime>) {
    println!("Starting BLE");
    runtime.spawn_background_task(|mut ctx| async move {
        println!("BLE task");
        let adapter = BLEAdapter::new().await.unwrap();
        let name = adapter.discover_hub_name().await.unwrap();
        println!("Found hub {}", name);
        ctx.run_on_main_thread(move |ctx| {
            if let Some(mut ble_state) = ctx.world.get_resource_mut::<BLEState>() {
                ble_state.adapter = Some(adapter);
            }
        })
        .await;
    });
}

pub struct BLEPlugin;

impl Plugin for BLEPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BLEState { adapter: None });
        app.add_systems(Startup, ble_startup_system);
    }
}
