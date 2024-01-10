use crate::bevy_tokio_tasks::TokioTasksRuntime;
use bevy::prelude::*;
use pybricks_ble::pybricks_hub::BLEAdapter;

#[derive(Resource)]
struct BLEState {
    adapter: Option<BLEAdapter>,
}

impl BLEState {}

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
