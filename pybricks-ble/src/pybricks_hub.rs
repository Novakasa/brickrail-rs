use btleplug::{
    api::{Central, CentralEvent, Manager as _, Peripheral, ScanFilter},
    platform::{Adapter, Manager},
};
use futures::StreamExt;
use uuid::Uuid;
pub const PYBRICKS_SERVICE_UUID: Uuid = Uuid::from_u128(0xc5f50001828046da89f46d8051e4aeef);

pub struct BLEAdapter {
    adapter: Adapter,
}

impl BLEAdapter {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = Manager::new().await?;
        let mut adapter_list = manager.adapters().await?;
        if adapter_list.is_empty() {
            return Err("No Bluetooth adapters".into());
        }
        let adapter = adapter_list.pop();
        Ok(BLEAdapter {
            adapter: adapter.unwrap(),
        })
    }

    pub async fn discover_hub_name(&self) -> Result<String, Box<dyn std::error::Error>> {
        let filter = ScanFilter::default();
        self.adapter.start_scan(filter).await?;
        let mut device = None;
        let mut events = self.adapter.events().await?;
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceUpdated(id) = event {
                println!("Device updated {:?}", id);
                let device_candidate = self.adapter.peripheral(&id).await?;
                let properties = device_candidate.properties().await?.unwrap();
                let name = properties.local_name;
                let services = properties.services;
                println!("Name: {:?} Services: {:?}", name, services);
                if services.contains(&PYBRICKS_SERVICE_UUID) && name.is_some() {
                    println!("Found Pybricks service for device {:?}", name);
                    device = Some(device_candidate);
                    break;
                }
            }
        }
        self.adapter.stop_scan().await?;
        Ok(device
            .unwrap()
            .properties()
            .await?
            .unwrap()
            .local_name
            .expect("Local name is None!"))
    }
}

pub struct PybricksHub {
    name: String,
}

impl PybricksHub {
    pub fn connect(&mut self) {
        println!("Connecting to {:?}", self.name);
    }
}
