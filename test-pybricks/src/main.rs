use std::path::Path;

use pybricks_ble::{
    io_hub::{IOHub, Input, SimulatedError},
    pybricks_hub::BLEAdapter,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .pretty()
        .init();
    let path = Path::new("../pybricks/programs/mpy/test_io.mpy");

    println!("path exists: {:?}", path.exists());
    let adapter = BLEAdapter::new().await?;
    let name = adapter.discover_hub_name().await?;
    println!("Found hub with name {:?}", name);
    let mut hub = IOHub::new(name);
    hub.discover(&adapter).await?;
    hub.connect().await?;
    tokio::time::sleep(std::time::Duration::from_secs(0)).await;
    hub.download_program(&path).await?;
    hub.start_program().await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    hub.queue_input(
        Input::rpc("respond", &vec![1, 3, 5, 7, 11, 13, 17, 19, 23, 29, 42])
            .with_error(SimulatedError::SkipAcknowledge),
    )
    .await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    hub.stop_program().await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.disconnect().await?;

    Ok(())
}
