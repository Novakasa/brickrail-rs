use std::{collections::BTreeSet, error::Error, path::Path, pin::Pin, u8, vec};

use btleplug::{
    api::{
        Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, PeripheralProperties,
        ScanFilter, ValueNotification, WriteType,
    },
    platform::{Adapter, Manager, Peripheral},
};
use futures::{Stream, StreamExt};
use tokio::sync::broadcast;
use tracing::{debug, error, info, trace};
use uuid::Uuid;

use crate::unpack_u32_little;
pub const PYBRICKS_SERVICE_UUID: Uuid = Uuid::from_u128(0xc5f50001828046da89f46d8051e4aeef);
pub const PYBRICKS_COMMAND_EVENT_UUID: Uuid = Uuid::from_u128(0xc5f50002828046da89f46d8051e4aeef);
pub const PYBRICKS_HUB_CAPABILITIES_UUID: Uuid =
    Uuid::from_u128(0xc5f50003828046da89f46d8051e4aeef);

fn pack_u32(n: u32) -> Vec<u8> {
    vec![
        (n & 0xFF) as u8,
        ((n >> 8) & 0xFF) as u8,
        ((n >> 16) & 0xFF) as u8,
        ((n >> 24) & 0xFF) as u8,
    ]
}

bitflags::bitflags! {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct HubStatus: u32 {
        const HIGH_CURRENT = 1;
        const BATTERY_LOW_VOLTAGE_SHUTDOWN = 2;
        const BATTERY_LOW_VOLTAGE_WARNING = 4;
        const BLE_ADVERTISING = 8;
        const BLE_LOW_SIGNAL = 16;
        const POWER_BUTTON_PRESSED = 32;
        const PROGRAM_RUNNING = 64;
        const SHUTDOWN = 128;
        const SHUTDOWN_REQUESTED = 256;
    }
}

#[derive(Debug)]
enum HubEvent {
    Status(HubStatus),
    STDOUT(Vec<u8>),
}

impl HubEvent {
    fn from_bytes(data: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        match data[0] {
            0 => Ok(HubEvent::Status(
                HubStatus::from_bits(unpack_u32_little(data[1..].to_vec())).unwrap(),
            )),
            1 => Ok(HubEvent::STDOUT(data[1..].to_vec())),
            _ => Err("Unknown event".into()),
        }
    }
}

struct HubCapabilities {
    max_write_size: u16,
    flags: u32,
    max_program_size: u32,
}

impl HubCapabilities {
    pub fn from_bytes(data: Vec<u8>) -> Self {
        // unpack according to "<HII":
        HubCapabilities {
            max_write_size: u16::from_le_bytes([data[0], data[1]]),
            flags: u32::from_le_bytes([data[2], data[3], data[4], data[5]]),
            max_program_size: u32::from_le_bytes([data[6], data[7], data[8], data[9]]),
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
        info!("Using BLE adapter {:?}", adapter);
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
        name_filter: Option<&str>,
    ) -> Result<Peripheral, Box<dyn Error>> {
        self.adapter.start_scan(ScanFilter::default()).await?;
        info!("Scanning...");
        let mut device = None;
        for device in self.adapter.peripherals().await? {
            if is_named_pybricks_hub(device.properties().await?, name_filter) {
                return Ok(device);
            }
        }
        let mut events = self.adapter.events().await?;
        while let Some(event) = events.next().await {
            if let CentralEvent::DeviceUpdated(id) = event {
                trace!("Device updated {:?}", id);
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
    name_filter: Option<&str>,
) -> bool {
    if properties.is_none() {
        return false;
    }
    let properties = properties.unwrap();
    let this_name = properties.local_name;
    if name_filter.is_some() && this_name.as_deref() != name_filter {
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

pub struct PybricksHub {
    name: Option<String>,
    client: Option<Peripheral>,
    chars: Option<HubCharacteristics>,
    capabilities: Option<HubCapabilities>,
    output_receiver: Option<broadcast::Receiver<u8>>,
    status_receiver: Option<broadcast::Receiver<HubStatus>>,
}

impl std::fmt::Display for PybricksHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            return write!(f, "{}", name);
        }
        write!(f, "{:?}", self.name)
    }
}

impl PybricksHub {
    pub fn new() -> Self {
        PybricksHub {
            name: None,
            client: None,
            chars: None,
            capabilities: None,
            output_receiver: None,
            status_receiver: None,
        }
    }

    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub async fn discover(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let adapter = BLEAdapter::new().await?;
        let device = adapter.discover_device(Some(name)).await?;
        self.client = Some(device);
        self.name = Some(name.to_string());

        let stream = self.client.as_ref().unwrap().notifications().await?;
        let (sender, receiver) = broadcast::channel(256);
        let (status_sender, status_receiver) = broadcast::channel(256);
        self.output_receiver = Some(receiver);
        self.status_receiver = Some(status_receiver);

        tokio::task::spawn(monitor_events(stream, sender, status_sender));

        Ok(())
    }

    pub fn subscribe_output(&mut self) -> Result<broadcast::Receiver<u8>, Box<dyn Error>> {
        Ok(self
            .output_receiver
            .as_ref()
            .ok_or("No output receiver")?
            .resubscribe())
    }

    pub fn subscribe_status(&mut self) -> Result<broadcast::Receiver<HubStatus>, Box<dyn Error>> {
        Ok(self
            .status_receiver
            .as_ref()
            .ok_or("No status receiver")?
            .resubscribe())
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let client = self.client.as_ref().ok_or("No client")?;
        debug!("Connecting to {:?}", client);
        client.connect().await?;
        client.discover_services().await?;
        self.chars = Some(HubCharacteristics::from_characteristics(
            client.characteristics(),
        )?);
        let capabilities = client
            .read(&self.chars.as_ref().unwrap().capabilities)
            .await?;
        self.capabilities = Some(HubCapabilities::from_bytes(capabilities));
        client
            .subscribe(&self.chars.as_ref().unwrap().command)
            .await?;
        info!("connected to pybricks hub {:?}!", self.name);
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        let client = self.client.as_ref().ok_or("No client")?;
        debug!("Disconnecting from {:}", self);
        client.disconnect().await?;
        info!(
            "disconnected from pybricks hub {:?}!",
            self.name.as_ref().unwrap()
        );
        Ok(())
    }

    async fn pb_command(&self, command: Command, data: &Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut command_data = vec![command as u8];
        command_data.extend(data);
        let client = self.client.as_ref().ok_or("No client")?;
        client
            .write(
                &self.chars.as_ref().unwrap().command,
                &command_data,
                WriteType::WithResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn write_stdin(&self, data: &Vec<u8>) -> Result<(), Box<dyn Error>> {
        debug!("Writing stdin to {:}: {:?}", self, data);
        self.pb_command(Command::WriteSTDIN, data).await
    }

    pub async fn download_program(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        info!("Downloading program to {:}", self);

        let data = std::fs::read(path)?;

        if data.len() > self.capabilities.as_ref().unwrap().max_program_size as usize {
            return Err("Program too large".into());
        }

        self.pb_command(Command::WriteUserProgramMeta, &pack_u32(0))
            .await?;

        let payload_size = self.capabilities.as_ref().unwrap().max_write_size as usize - 5;

        for (i, chunk) in data.chunks(payload_size).enumerate() {
            let mut data = pack_u32((i * payload_size) as u32);
            data.extend_from_slice(chunk);
            self.pb_command(Command::WriteUserRam, &data).await?;
        }

        self.pb_command(Command::WriteUserProgramMeta, &pack_u32(data.len() as u32))
            .await?;

        info!("Downloaded program finished for {:?}", self.name);

        Ok(())
    }

    pub async fn start_program(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting program on {:?}", self.name);
        self.pb_command(Command::StartUserProgram, &vec![]).await
    }

    pub async fn stop_program(&self) -> Result<(), Box<dyn Error>> {
        info!("Stopping program on {:?}", self.name);
        self.pb_command(Command::StopUserProgram, &vec![]).await
    }
}

async fn monitor_events(
    mut stream: Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
    output_sender: broadcast::Sender<u8>,
    status_sender: broadcast::Sender<HubStatus>,
) {
    debug!("Listening for notifications");
    while let Some(data) = stream.next().await {
        match data.uuid {
            PYBRICKS_COMMAND_EVENT_UUID => {
                if let Ok(event) = HubEvent::from_bytes(data.value) {
                    match event {
                        HubEvent::STDOUT(data) => {
                            for byte in data {
                                if output_sender.send(byte).is_err() {
                                    println!("Failed to send output byte {}", byte);
                                }
                            }
                        }
                        HubEvent::Status(status) => {
                            if status_sender.send(status.clone()).is_err() {
                                println!("Failed to send status {:?}", status);
                            }
                        }
                    }
                }
            }
            _ => {
                error!("Unknown event");
            }
        }
    }
    debug!("Done listening for notifications");
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unpack_capabilities() {
        let data = vec![164, 1, 244, 26, 0, 0, 15, 1, 0, 0];
        let caps = HubCapabilities::from_bytes(data);
        assert_eq!(caps.max_write_size, 420);
        assert_eq!(caps.flags, 6900);
        assert_eq!(caps.max_program_size, 271);
    }

    #[test]
    fn test_pack_unpack() {
        let n = 420;
        let packed = pack_u32(n);
        assert_eq!(packed, vec![164, 1, 0, 0]);
        let unpacked = unpack_u32_little(packed);
        assert_eq!(n, unpacked);
    }
}
