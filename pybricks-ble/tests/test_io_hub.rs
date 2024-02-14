use std::path::Path;

use pybricks_ble::{
    io_hub::{IOEvent, IOHub, IOMessage, Input, SimulatedError},
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
    hub.connect(&name).await.unwrap();
    hub
}

async fn hub_with_io_program() -> IOHub {
    let hub = get_and_connect_hub().await;
    let path = Path::new("../pybricks/programs/mpy/test_io.mpy");
    hub.download_program(&path).await.unwrap();
    hub
}

const TEST_ERR: SimulatedError = SimulatedError::Modify(4);

async fn test_io(hub: &mut IOHub) {
    hub.start_program().await.unwrap();
    hub.set_simulated_output_error(TEST_ERR).await.unwrap();
    hub.queue_input(Input::rpc("respond", &vec![29, 42]).with_error(TEST_ERR))
        .unwrap();
    match hub.wait_for_data_event_with_id(57).await.unwrap() {
        IOEvent::Message(IOMessage::Data { id, data }) => {
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
        .unwrap();
    hub.queue_input(Input::rpc("add_to_counter", &vec![5]).with_error(TEST_ERR))
        .unwrap();
    hub.queue_input(Input::rpc("get_counter", &vec![]).with_error(TEST_ERR))
        .unwrap();
    assert_eq!(hub.wait_for_data(42).await.unwrap(), vec![38]);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.stop_program().await.unwrap();
}

async fn test_io_hub_many_messages(hub: &mut IOHub, err: SimulatedError) {
    hub.start_program().await.unwrap();
    hub.queue_input(Input::rpc("set_counter", &vec![0]))
        .unwrap();

    let num_messages = 12;
    for _i in 0..num_messages {
        hub.queue_input(Input::rpc("add_to_counter", &vec![1]).with_error(err))
            .unwrap();
    }
    hub.queue_input(Input::rpc("get_counter", &vec![]).with_error(err))
        .unwrap();
    let result = hub.wait_for_data(42).await.unwrap()[0];
    assert_eq!(result as u32, num_messages % 256);
    hub.stop_program().await.unwrap();
}

#[test_log::test(tokio::test)]
#[ignore]
async fn test_io_hub() {
    let mut hub = hub_with_io_program().await;

    test_io_hub_counter(&mut hub).await;

    test_io(&mut hub).await;

    test_io_hub_many_messages(&mut hub, SimulatedError::Modify(4)).await;

    hub.disconnect().await.unwrap();
}

#[test_log::test(tokio::test)]
#[ignore]
async fn test_io_2_hubs() {
    let mut hub1 = hub_with_io_program().await;
    let mut hub2 = hub_with_io_program().await;

    // in sequence
    test_io_hub_many_messages(&mut hub1, SimulatedError::Modify(3)).await;
    test_io_hub_many_messages(&mut hub2, SimulatedError::Modify(2)).await;

    // in parallel
    let ((), ()) = tokio::join!(
        test_io_hub_many_messages(&mut hub1, SimulatedError::Modify(3)),
        test_io_hub_many_messages(&mut hub2, SimulatedError::Modify(2)),
    );

    hub1.disconnect().await.unwrap();
    hub2.disconnect().await.unwrap();
}
