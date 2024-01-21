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

async fn hub_with_io_program() -> IOHub {
    let hub = get_and_connect_hub().await;
    let path = Path::new("../pybricks/programs/mpy/test_io.mpy");
    hub.download_program(&path).await.unwrap();
    hub
}

const TEST_ERR: SimulatedError = SimulatedError::SkipAcknowledge;

async fn test_io(hub: &mut IOHub) {
    hub.start_program().await.unwrap();
    hub.set_simulated_output_error(TEST_ERR).await.unwrap();
    hub.queue_input(Input::rpc("respond", &vec![29, 42]).with_error(TEST_ERR))
        .await
        .unwrap();
    match hub.wait_for_data_event_with_id(57).await.unwrap() {
        IOEvent::Data { id, data } => {
            assert_eq!(id, 57);
            assert_eq!(data, vec![29, 42]);
        }
        _ => panic!("Unexpected input"),
    }
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.stop_program().await.unwrap();
}

async fn test_io_hub_counter(hub: &mut IOHub) {
    hub.start_program().await.unwrap();
    hub.set_simulated_output_error(TEST_ERR).await.unwrap();
    hub.queue_input(Input::rpc("set_counter", &vec![33]).with_error(TEST_ERR))
        .await
        .unwrap();
    hub.queue_input(Input::rpc("add_to_counter", &vec![5]).with_error(TEST_ERR))
        .await
        .unwrap();
    hub.queue_input(Input::rpc("get_counter", &vec![]).with_error(TEST_ERR))
        .await
        .unwrap();
    assert_eq!(hub.wait_for_data(42).await.unwrap(), vec![38]);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.stop_program().await.unwrap();
}

#[test_log::test(tokio::test)]
#[ignore]
async fn test_io_hub() {
    let mut hub = hub_with_io_program().await;

    test_io_hub_counter(&mut hub).await;

    test_io(&mut hub).await;

    hub.disconnect().await.unwrap();
}
