use std::path::Path;

use pybricks_ble::{
    io_hub::{IOEvent, IOHub, Input, SimulatedError},
    pybricks_hub::BLEAdapter,
};

async fn get_and_connect_hub() -> IOHub {
    let adapter = BLEAdapter::new().await.unwrap();
    let name = adapter.discover_hub_name().await.unwrap();
    println!("Found hub with name {:?}", name);
    let mut hub = IOHub::new(name);
    hub.discover(&adapter).await.unwrap();
    hub.connect().await.unwrap();
    hub
}

#[test_log::test(tokio::test)]
async fn test_io_hub() {
    let path = Path::new("../pybricks/programs/mpy/test_io.mpy");

    let hub = get_and_connect_hub().await;

    hub.download_program(&path).await.unwrap();
    hub.start_program().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    hub.set_simulated_output_error(SimulatedError::None)
        .await
        .unwrap();
    hub.queue_input(Input::rpc("respond", &vec![29, 42]).with_error(SimulatedError::None))
        .await
        .unwrap();
    match hub.wait_for_data_id(57).await.unwrap() {
        IOEvent::Data { id, data } => {
            assert_eq!(id, 57);
            assert_eq!(data, vec![29, 42]);
        }
        _ => panic!("Unexpected input"),
    }
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    hub.stop_program().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.disconnect().await.unwrap();
}
