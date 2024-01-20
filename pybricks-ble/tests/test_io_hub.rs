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
    hub.set_simulated_output_error(SimulatedError::SkipAcknowledge)
        .await
        .unwrap();
    hub.queue_input(
        Input::rpc("respond", &vec![29, 42]).with_error(SimulatedError::SkipAcknowledge),
    )
    .await
    .unwrap();
    match hub.wait_for_data_event_with_id(57).await.unwrap() {
        IOEvent::Data { id, data } => {
            assert_eq!(id, 57);
            assert_eq!(data, vec![29, 42]);
        }
        _ => panic!("Unexpected input"),
    }
    hub.stop_program().await.unwrap();
    hub.disconnect().await.unwrap();
}

#[test_log::test(tokio::test)]
async fn test_io_hub_counter() {
    let path = Path::new("../pybricks/programs/mpy/test_io.mpy");

    let hub = get_and_connect_hub().await;

    hub.download_program(&path).await.unwrap();
    hub.start_program().await.unwrap();
    hub.set_simulated_output_error(SimulatedError::SkipAcknowledge)
        .await
        .unwrap();
    hub.queue_input(
        Input::rpc("set_counter", &vec![33]).with_error(SimulatedError::SkipAcknowledge),
    )
    .await
    .unwrap();
    hub.queue_input(
        Input::rpc("add_to_counter", &vec![5]).with_error(SimulatedError::SkipAcknowledge),
    )
    .await
    .unwrap();
    hub.queue_input(Input::rpc("get_counter", &vec![]).with_error(SimulatedError::SkipAcknowledge))
        .await
        .unwrap();
    assert_eq!(hub.wait_for_data(42).await.unwrap(), vec![38]);
    hub.stop_program().await.unwrap();
    hub.disconnect().await.unwrap();
}
