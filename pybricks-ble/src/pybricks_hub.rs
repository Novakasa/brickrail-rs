use std::{
    collections::BTreeSet,
    error::Error,
    path::{self, Path},
};

use btleplug::{
    api::{
        Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, PeripheralProperties,
        ScanFilter, WriteType,
    },
    platform::{Adapter, Manager, Peripheral, PeripheralId},
};
use futures::StreamExt;
use tokio::io::AsyncReadExt;
use uuid::Uuid;
pub const PYBRICKS_SERVICE_UUID: Uuid = Uuid::from_u128(0xc5f50001828046da89f46d8051e4aeef);
pub const PYBRICKS_COMMAND_EVENT_UUID: Uuid = Uuid::from_u128(0xc5f50002828046da89f46d8051e4aeef);
pub const PYBRICKS_HUB_CAPABILITIES_UUID: Uuid =
    Uuid::from_u128(0xc5f50003828046da89f46d8051e4aeef);

struct HubCapabilities {
    max_char_size: u16,
    flags: u32,
    max_write_size: u32,
}

impl HubCapabilities {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        // unpack according to "<HII":
        HubCapabilities {
            max_char_size: u16::from_le_bytes([data[0], data[1]]),
            flags: u32::from_le_bytes([data[2], data[3], data[4], data[5]]),
            max_write_size: u32::from_le_bytes([data[6], data[7], data[8], data[9]]),
        }
    }
}

struct HubCharacteristics {
    command: Characteristic,
    capabilities: Characteristic,
}

impl HubCharacteristics {
    pub fn from_characteristics(
        characteristics: BTreeSet<Characteristic>,
    ) -> Result<Self, Box<dyn Error>> {
        let command = characteristics
            .iter()
            .find(|c| c.uuid == PYBRICKS_COMMAND_EVENT_UUID)
            .ok_or("No command characteristic")?;
        let capabilities = characteristics
            .iter()
            .find(|c| c.uuid == PYBRICKS_HUB_CAPABILITIES_UUID)
            .ok_or("No capabilities characteristic")?;
        Ok(HubCharacteristics {
            command: command.clone(),
            capabilities: capabilities.clone(),
        })
    }
}

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
        self.adapter.start_scan(ScanFilter::default()).await?;
        println!("Scanning...");
        let mut device = None;
        for device in self.adapter.peripherals().await? {
            if is_named_pybricks_hub(device.properties().await?, name_filter) {
                return Ok(device);
            }
        }
        let mut events = self.adapter.events().await?;
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceUpdated(id) = event {
                println!("Device updated {:?}", id);
                let device_candidate = self.adapter.peripheral(&id).await?;
                if is_named_pybricks_hub(device_candidate.properties().await?, name_filter) {
                    device = Some(device_candidate);
                    break;
                }
            }
        }
        self.adapter.stop_scan().await.unwrap();
        Ok(device.unwrap())
    }
}

fn is_named_pybricks_hub(
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
    name: String,
    client: Option<Peripheral>,
    chars: Option<HubCharacteristics>,
    capabilities: Option<HubCapabilities>,
}

impl PybricksHub {
    pub fn new(name: String, client: Peripheral) -> Self {
        PybricksHub {
            name: name,
            client: Some(client),
            chars: None,
            capabilities: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Connecting to {:?}", self.name);
        let client = self.client.as_ref().ok_or("No client")?;
        client.connect().await?;
        println!("connected!");
        client.discover_services().await?;
        self.chars = Some(HubCharacteristics::from_characteristics(
            client.characteristics(),
        )?);
        let capabilities = client
            .read(&self.chars.as_ref().unwrap().capabilities)
            .await?;
        self.capabilities = Some(HubCapabilities::from_bytes(capabilities));
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        println!("Disconnecting from {:?}", self.name);
        let client = self.client.as_ref().ok_or("No client")?;
        client.disconnect().await?;
        Ok(())
    }

    pub async fn write_stdin(&self, mut data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        println!("Writing stdin to {:?}", self.name);
        data.insert(0, Command::WriteSTDIN as u8);
        let client = self.client.as_ref().ok_or("No client")?;
        client
            .write(
                &self.chars.as_ref().unwrap().command,
                &data,
                WriteType::WithResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn download_program(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        println!("Downloading program to {:?}", self.name);
        let client = self.client.as_ref().ok_or("No client")?;
        Ok(())
    }

    pub async fn start_program(&self) -> Result<(), Box<dyn Error>> {
        println!("Starting program on {:?}", self.name);
        self.client
            .as_ref()
            .ok_or("No client")?
            .write(
                &self.chars.as_ref().unwrap().command,
                &[Command::StartUserProgram as u8],
                WriteType::WithResponse,
            )
            .await?;
        Ok(())
    }
    pub async fn stop_program(&self) -> Result<(), Box<dyn Error>> {
        println!("Stopping program on {:?}", self.name);
        self.client
            .as_ref()
            .ok_or("No client")?
            .write(
                &self.chars.as_ref().unwrap().command,
                &[Command::StopUserProgram as u8],
                WriteType::WithResponse,
            )
            .await?;
        Ok(())
    }
}
