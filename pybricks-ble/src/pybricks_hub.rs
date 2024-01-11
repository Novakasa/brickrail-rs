use std::error::Error;

use btleplug::{
    api::{Central, CentralEvent, Manager as _, Peripheral as _, PeripheralProperties, ScanFilter},
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
        name_filter: Option<&String>,
    ) -> Result<Peripheral, Box<dyn Error>> {
        let filter = ScanFilter::default();
        self.adapter.start_scan(filter).await?;
        println!("Scanning...");
        let mut device = None;
        for device in self.adapter.peripherals().await? {
            if is_pybricks_device(device.properties().await?, name_filter) {
                return Ok(device);
            }
        }
        let mut events = self.adapter.events().await?;
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceUpdated(id) = event {
                println!("Device updated {:?}", id);
                let device_candidate = self.adapter.peripheral(&id).await?;
                if is_pybricks_device(device_candidate.properties().await?, name_filter) {
                    device = Some(device_candidate);
                    break;
                }
            }
        }
        self.adapter.stop_scan().await.unwrap();
        Ok(device.unwrap())
    }
}

fn is_pybricks_device(
    properties: Option<PeripheralProperties>,
    name_filter: Option<&String>,
) -> bool {
    if properties.is_none() {
        return false;
    }
    let properties = properties.unwrap();
    let this_name = properties.local_name.clone();
    if name_filter.is_some() && this_name.as_ref() != name_filter {
        return false;
    }
    if !properties.services.contains(&PYBRICKS_SERVICE_UUID) {
        return false;
    }
    if this_name.is_none() {
        return false;
    }
    return true;
}

pub struct BLEClient {
    adapter: Adapter,
    id: PeripheralId,
}

pub struct PybricksHub {
    pub name: String,
    pub client: Option<Peripheral>,
}

impl PybricksHub {
    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        println!("Connecting to {:?}", self.name);
        self.client.as_ref().ok_or("No client")?.connect().await?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        println!("Disconnecting from {:?}", self.name);
        self.client
            .as_ref()
            .ok_or("No client")?
            .disconnect()
            .await?;
        Ok(())
    }
}
