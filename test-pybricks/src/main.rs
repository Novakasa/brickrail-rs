use std::path::Path;

use pybricks_ble::{io_hub::IOHub, pybricks_hub::BLEAdapter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("../pybricks/programs/mpy/test_io1.mpy");

    println!("path exists: {:?}", path.exists());
    let adapter = BLEAdapter::new().await?;
    let name = adapter.discover_hub_name().await?;
    println!("Found hub with name {:?}", name);
    let mut hub = IOHub::new(name);
    hub.discover(&adapter).await?;
    hub.connect().await?;
    tokio::time::sleep(std::time::Duration::from_secs(0)).await;
    println!("cwd: {:?}", std::env::current_dir()?);

    println!("path: {:?}", path);
    hub.download_program(&path).await?;
    hub.start_program().await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    hub.stop_program().await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    hub.disconnect().await?;

    Ok(())
}
