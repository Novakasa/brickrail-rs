use pybricks_ble::pybricks_hub::{BLEAdapter, PybricksHub};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let adapter = BLEAdapter::new().await?;
    let name = adapter.discover_hub_name().await?;
    println!("Found hub with name {:?}", name);
    // tokio::time::sleep(std::time::Duration::from_secs(0)).await;

    let device = adapter.discover_device(Some(&name)).await?;
    let mut hub = PybricksHub::new(name, device);
    hub.connect().await?;
    // wait 5 seconds
    tokio::time::sleep(std::time::Duration::from_secs(0)).await;
    hub.start_program().await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    hub.stop_program().await?;
    hub.disconnect().await?;
    Ok(())
}
