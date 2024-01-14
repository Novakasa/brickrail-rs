use futures::channel::mpsc;
use tokio::sync::broadcast;

use crate::{
    pybricks_hub::{BLEAdapter, PybricksHub},
    unpack_u16_little,
};
use std::{
    error::Error,
    path::Path,
    sync::{Arc, Mutex},
};

const IN_ID_END: u8 = 10;
const IN_ID_MSG_ACK: u8 = 6;
const IN_ID_RPC: u8 = 17;
const IN_ID_SYS: u8 = 18;
const IN_ID_STORE: u8 = 19;
const IN_ID_MSG_ERR: u8 = 21;

const OUT_ID_END: u8 = 10;
const OUT_ID_MSG_ACK: u8 = 6;
const OUT_ID_DATA: u8 = 17;
const OUT_ID_SYS: u8 = 18;
const OUT_ID_MSG_ERR: u8 = 21;
const OUT_ID_DUMP: u8 = 20;

const SYS_CODE_STOP: u8 = 0;
const SYS_CODE_READY: u8 = 1;
const SYS_CODE_ALIVE: u8 = 2;
const SYS_CODE_VERSION: u8 = 3;

fn xor_checksum(data: &[u8]) -> u8 {
    let mut checksum = 0xFF;
    for byte in data {
        checksum ^= byte;
    }
    checksum
}

#[derive(Debug, PartialEq, Eq)]
enum OutputType {
    None,
    MsgAck,
    Data,
    Sys,
    MsgErr,
    Dump,
}

impl OutputType {
    fn from_byte(byte: u8) -> Result<Self, Box<dyn Error>> {
        match byte {
            OUT_ID_MSG_ACK => Ok(OutputType::MsgAck),
            OUT_ID_DATA => Ok(OutputType::Data),
            OUT_ID_SYS => Ok(OutputType::Sys),
            OUT_ID_MSG_ERR => Ok(OutputType::MsgErr),
            OUT_ID_DUMP => Ok(OutputType::Dump),
            _ => Err("Unknown output type".into()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum InputType {
    MsgAck,
    RPC,
    Sys,
    Store,
    MsgErr,
}

impl InputType {
    fn to_u8(&self) -> u8 {
        match self {
            InputType::MsgAck => IN_ID_MSG_ACK,
            InputType::RPC => IN_ID_RPC,
            InputType::Sys => IN_ID_SYS,
            InputType::Store => IN_ID_STORE,
            InputType::MsgErr => IN_ID_MSG_ERR,
        }
    }
}

struct Output {
    output_type: OutputType,
    data: Vec<u8>,
}

impl Output {
    fn from_bytes(data: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let output_type = OutputType::from_byte(data[0])?;
        let data = data[1..].to_vec();
        Ok(Output {
            output_type: output_type,
            data: data,
        })
    }
}

struct Input {
    input_type: InputType,
    data: Vec<u8>,
}

impl Input {
    fn to_bytes(&self, input_id: u8) -> Vec<u8> {
        let mut data = vec![self.input_type.to_u8()];
        data.extend_from_slice(&self.data);
        data.insert(0, data.len() as u8);
        if ![InputType::MsgAck, InputType::MsgErr].contains(&self.input_type) {
            let checksum = xor_checksum(&data);
            data.push(input_id);
            data.push(checksum);
        }
        data.push(IN_ID_END);
        data
    }
}

pub struct IOState {
    line_buffer: Vec<u8>,
    line_sender: Option<broadcast::Sender<String>>,
    print_output: bool,
    msg_len: Option<usize>,
    output_buffer: Vec<u8>,
    long_message: bool,
    next_output_id: u8,
    input_sender: Option<mpsc::Sender<Input>>,
}

impl IOState {
    pub fn new() -> Self {
        IOState {
            line_buffer: vec![],
            line_sender: None,
            print_output: true,
            msg_len: None,
            output_buffer: vec![],
            long_message: false,
            next_output_id: 0,
            input_sender: None,
        }
    }

    pub fn subscribe_lines(&mut self) -> broadcast::Receiver<String> {
        if let Some(sender) = self.line_sender.as_ref() {
            return sender.subscribe();
        }
        let (sender, receiver) = broadcast::channel(256);
        self.line_sender = Some(sender);
        receiver
    }

    fn on_output_byte_received(&mut self, byte: u8) {
        self.update_output_buffer(byte);
        self.update_line_buffer(byte);
    }

    fn update_output_buffer(&mut self, byte: u8) {
        if self.msg_len.is_none() {
            self.msg_len = Some(byte as usize);
            println!("message length: {:?}", self.msg_len);
            return;
        }

        if self.output_buffer == vec![OUT_ID_DUMP] {
            self.msg_len = Some(unpack_u16_little([self.msg_len.unwrap() as u8, byte]) as usize);
            println!("dump length: {:?}", self.msg_len);
            self.long_message = true;
            return;
        }

        if self.output_buffer.len() == self.msg_len.unwrap()
            && byte == OUT_ID_END
            && self.output_buffer[0] < 32
        {
            println!("handling message...");
            self.handle_output();
            self.clear();
            return;
        }

        self.output_buffer.push(byte);
        println!("output buffer: {:?}", self.output_buffer)
    }

    fn handle_output(&mut self) {
        let output_type = OutputType::from_byte(self.output_buffer[0]).unwrap();
        match output_type {
            OutputType::MsgAck => {
                println!("Message acknowledged");
                return;
            }
            OutputType::MsgErr => {
                println!("Message error");
                return;
            }
            OutputType::Dump => {
                println!("Dump");
                return;
            }
            _ => {}
        }

        let checksum = self.output_buffer[self.output_buffer.len() - 1];
        let output_id = self.output_buffer[self.output_buffer.len() - 2];
        let data = &self.output_buffer[1..self.output_buffer.len() - 2];
        let expected_checksum = xor_checksum(&self.output_buffer[0..self.output_buffer.len() - 2]);

        if output_id == self.next_output_id.wrapping_sub(1) {
            // This is a retransmission of the previous message.
            // acknowledge it and ignore it.
            println!("Retransmission of message {:?}", output_id);
            return;
        }
        if checksum != expected_checksum || output_id != self.next_output_id {
            println!(
                "Checksum mismatch: expected {:?}, got {:?}",
                expected_checksum, checksum
            );
            // send NAK
            return;
        }

        // acknowledge the message
        self.next_output_id = self.next_output_id.wrapping_add(1);
        println!("Message success: {:?}", data);
        println!("Next output ID: {:?}", self.next_output_id);
    }

    fn update_line_buffer(&mut self, byte: u8) {
        self.line_buffer.push(byte);
        if self.line_buffer.ends_with(&vec![13, 10]) && self.line_buffer[1] >= 32 {
            if let Ok(line) = std::str::from_utf8(&self.line_buffer) {
                if let Some(sender) = self.line_sender.as_ref() {
                    sender.send(line.to_string()).unwrap();
                }
                if self.print_output {
                    print!("[Hub STDOUT] {}", line);
                }
                self.clear();
            }
        }
    }

    fn clear(&mut self) {
        self.line_buffer.clear();
        self.msg_len = None;
        self.output_buffer.clear();
        self.long_message = false;
    }
}

pub struct IOHub {
    hub: Arc<Mutex<PybricksHub>>,
    io_state: Arc<Mutex<IOState>>,
}

impl IOHub {
    pub fn new(name: String) -> Self {
        IOHub {
            hub: Arc::new(Mutex::new(PybricksHub::new(name.into()))),
            io_state: Arc::new(Mutex::new(IOState::new())),
        }
    }

    pub async fn discover(&self, adapter: &BLEAdapter) -> Result<(), Box<dyn Error>> {
        let mut hub = self.hub.lock().unwrap();
        hub.discover(adapter).await?;
        let mut output_receiver = hub.subscribe_output()?;
        let io_state = self.io_state.clone();
        tokio::spawn(async move {
            while let Ok(byte) = output_receiver.recv().await {
                let mut io_state = io_state.lock().unwrap();
                io_state.on_output_byte_received(byte);
            }
        });
        Ok(())
    }

    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let mut hub = self.hub.lock().unwrap();
        hub.connect().await?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().unwrap();
        hub.disconnect().await?;
        Ok(())
    }

    pub async fn download_program(&self, name: &Path) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().unwrap();
        hub.download_program(name).await?;
        Ok(())
    }

    pub async fn start_program(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().unwrap();
        hub.start_program().await?;
        Ok(())
    }

    pub async fn stop_program(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().unwrap();
        hub.stop_program().await?;
        Ok(())
    }
}
