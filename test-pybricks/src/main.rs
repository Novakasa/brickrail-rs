use btleplug::api::bleuuid::BleUuid;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::Manager;
use futures::StreamExt;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;
    let adapter_list = manager.adapters().await?;
    if adapter_list.is_empty() {
        eprintln!("No Bluetooth adapters");
    }
    println!("Found {} adapters", adapter_list.len());
    let adapter = adapter_list.first().unwrap();
    // pybricks uuid: 'c5f50001-8280-46da-89f4-6d8051e4aeef'
    let pybricks_service_uuid = Uuid::from_u128(0xc5f50001828046da89f46d8051e4aeef);
    let filter = ScanFilter {
        services: vec![pybricks_service_uuid],
    };

    let mut events = adapter.events().await?;
    adapter.start_scan(filter).await?;
    let mut device = None;
    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                println!("DeviceDiscovered: {:?}", id);
                loop {
                    // wait 1 second for name??
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    let device_candidate = adapter.peripheral(&id).await?;
                    let name = device_candidate.properties().await?.unwrap().local_name;
                    println!("Device name: {:?}", name);
                    if name != None {
                        device = Some(device_candidate);
                        break;
                    }
                }
                adapter.stop_scan().await?;
                break;
            }
            _ => {}
        }
    }
    println!(
        "Connecting to {:?}",
        device.unwrap().properties().await?.unwrap().local_name
    );
    Ok(())
}
