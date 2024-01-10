use std::error::Error;

use btleplug::{
    api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter},
    platform::{Adapter, Manager, Peripheral, PeripheralId},
};
use futures::StreamExt;
use uuid::Uuid;
pub const PYBRICKS_SERVICE_UUID: Uuid = Uuid::from_u128(0xc5f50001828046da89f46d8051e4aeef);
pub const PYBRICKS_COMMAND_EVENT_UUID: Uuid = Uuid::from_u128(0xc5f50002828046da89f46d8051e4aeef);
pub const PYBRICKS_HUB_CAPABILITIES_UUID: Uuid =
    Uuid::from_u128(0xc5f50003828046da89f46d8051e4aeef);

enum Command {
    StopUserProgram,
    StartUserProgram,
    StartRepl,
    WriteUserProgramMeta,
    WriteUserRam,
    RebootToUpdateMode,
    WriteSTDIN,
}

enum Event {
    StatusReport,
    WriteSTDOUT,
}

enum StatusFlag {
    BatteryLowVoltageWarning,
    BatteryLowVoltageShutdown,
    BatteryHighCurrent,
    BLEAdvertising,
    BLELowSignal,
    PowerButtonPressed,
    UserProgramRunning,
    Shutdown,
    ShutdownRequested,
}

pub struct BLEAdapter {
    adapter: Adapter,
}

impl BLEAdapter {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = Manager::new().await?;
        let adapter_list = manager.adapters().await?;
        if adapter_list.is_empty() {
            return Err("No Bluetooth adapters".into());
        }
        let adapter = adapter_list.first();
        println!("Using adapter {:?}", adapter);
        Ok(BLEAdapter {
            adapter: adapter.unwrap().clone(),
        })
    }

    pub async fn discover_hub_name(&self) -> Result<String, Box<dyn std::error::Error>> {
        let device = self.discover_device(None).await?;
        Ok(device
            .properties()
            .await?
            .ok_or("No properties")?
            .local_name
            .ok_or("Local name is None!")?)
    }

    pub async fn discover_device(
        &self,
        name_filter: Option<String>,
    ) -> Result<Peripheral, Box<dyn Error>> {
        let filter = ScanFilter::default();
        self.adapter.start_scan(filter).await?;
        let mut device = None;
        let mut events = self.adapter.events().await?;
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceUpdated(id) = event {
                let device_candidate = self.adapter.peripheral(&id).await?;
                let properties = device_candidate.properties().await?;
                let properties = if properties.is_none() {
                    continue;
                } else {
                    properties.unwrap()
                };
                let this_name = properties.local_name.clone();
                if name_filter.is_some() && this_name == name_filter {
                    continue;
                }
                if !properties.services.contains(&PYBRICKS_SERVICE_UUID) {
                    continue;
                }
                if this_name.is_none() {
                    continue;
                }
                device = Some(device_candidate);
                break;
            }
        }
        self.adapter.stop_scan().await.unwrap();
        Ok(device.unwrap())
    }
}

pub struct BLEClient {
    adapter: Adapter,
    id: PeripheralId,
}

pub struct PybricksHub {
    name: String,
    client: Option<Peripheral>,
}

impl PybricksHub {
    pub fn connect(&mut self) {
        println!("Connecting to {:?}", self.name);
    }
}
