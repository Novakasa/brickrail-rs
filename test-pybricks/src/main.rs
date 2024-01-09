use pybricks_ble::pybricks_hub::discover_hub_name;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = discover_hub_name().await?;
    println!("Found hub with name {:?}", name);
    Ok(())
}
