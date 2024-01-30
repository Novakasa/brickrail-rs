use std::path::Path;

use bevy::log::info;
use pybricks_ble::{
    io_hub::{IOEvent, IOHub, Input, SimulatedError},
    pybricks_hub::BLEAdapter,
};

async fn get_and_connect_hub() -> IOHub {
    let adapter = BLEAdapter::new().await.unwrap();
    let name = adapter.discover_hub_name().await.unwrap();
    println!("Found hub with name {:?}", name);
    let mut hub = IOHub::new();
    let mut events_receiver = hub.subscribe_events();
    hub.discover(name.as_str()).await.unwrap();
    tokio::task::spawn(async move {
        while let Ok(event) = events_receiver.recv().await {
            println!("Event: {:?}", event);
        }
    });
    hub.connect().await.unwrap();
    hub
}

async fn train_hub() -> IOHub {
    let hub = get_and_connect_hub().await;
    let path = Path::new("../pybricks/programs/mpy/smart_train.mpy");
    hub.download_program(&path).await.unwrap();
    hub
}

#[test_log::test(tokio::test)]
async fn test_route() {
    let hub = &mut train_hub().await;
    println!("program running");
    std::thread::sleep(std::time::Duration::from_secs(5));
    hub.disconnect().await.unwrap();
}
