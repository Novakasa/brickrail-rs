use futures::lock::Mutex;
use tokio::{
    sync::{
        broadcast,
        mpsc::{self, UnboundedSender},
    },
    time::timeout,
};

use crate::{
    pybricks_hub::{BLEAdapter, PybricksHub},
    unpack_u16_little,
};
use std::{error::Error, path::Path, sync::Arc, time::Duration};

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

    fn expect_response(&self) -> bool {
        match self {
            OutputType::MsgAck => false,
            OutputType::MsgErr => false,
            OutputType::Dump => false,
            _ => true,
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

#[derive(Debug)]
struct Output {
    output_type: OutputType,
    data: Vec<u8>,
    received_checksum: Option<u8>,
    computed_checksum: Option<u8>,
    output_id: u8,
}

impl Output {
    fn from_bytes(mut data: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        let output_type = OutputType::from_byte(data[0])?;
        let mut received_checksum = None;
        let mut computed_checksum = None;
        if output_type.expect_response() {
            received_checksum = data.pop();
            computed_checksum = Some(xor_checksum(&data));
        }
        let output_id = data.pop().unwrap();
        let data = data[1..].to_vec();
        Ok(Output {
            output_type,
            data,
            received_checksum,
            computed_checksum,
            output_id,
        })
    }

    fn validate_checksum(&self) -> bool {
        self.received_checksum == self.computed_checksum
    }
}

#[derive(Debug)]
pub struct Input {
    input_type: InputType,
    data: Vec<u8>,
}

impl Input {
    pub fn acknowledge(output_id: u8) -> Self {
        Input {
            input_type: InputType::MsgAck,
            data: vec![output_id],
        }
    }

    pub fn msg_err(input_id: u8) -> Self {
        Input {
            input_type: InputType::MsgErr,
            data: vec![input_id],
        }
    }

    fn to_bytes(&self, input_id: u8) -> Vec<u8> {
        let mut data = vec![self.input_type.to_u8()];
        data.extend_from_slice(&self.data);
        data.insert(0, data.len() as u8);
        if self.expect_response() {
            data.push(input_id);
            let checksum = xor_checksum(&data);
            data.push(checksum);
        }
        data.push(IN_ID_END);
        data
    }

    fn expect_response(&self) -> bool {
        match self.input_type {
            InputType::MsgAck => false,
            InputType::MsgErr => false,
            _ => true,
        }
    }
}

pub struct IOState {
    line_buffer: Vec<u8>,
    line_sender: Option<broadcast::Sender<String>>,
    print_lines: bool,
    output_len: Option<usize>,
    output_buffer: Vec<u8>,
    long_output: bool,
    next_output_id: u8,
    response_sender: UnboundedSender<Output>,
    input_queue_sender: UnboundedSender<Input>,
}

impl IOState {
    pub fn new(input_sender: UnboundedSender<Vec<u8>>) -> Self {
        let (response_sender, mut response_receiver) = mpsc::unbounded_channel();
        let (input_queue_sender, mut input_queue_receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::input_queue_task(
            input_queue_receiver,
            input_sender,
            response_receiver,
        ));

        IOState {
            line_buffer: vec![],
            line_sender: None,
            print_lines: true,
            output_len: None,
            output_buffer: vec![],
            long_output: false,
            next_output_id: 0,
            response_sender: response_sender,
            input_queue_sender: input_queue_sender,
        }
    }

    pub fn queue_input(&self, input: Input) -> Result<(), Box<dyn Error>> {
        Ok(self.input_queue_sender.send(input)?)
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
        let just_cleared = self.update_output_buffer(byte);
        if !just_cleared {
            self.update_line_buffer(byte);
        }
    }

    fn update_output_buffer(&mut self, byte: u8) -> bool {
        if self.output_len.is_none() {
            self.output_len = Some(byte as usize);
            println!("message length: {:?}", self.output_len);
            return false;
        }

        if self.output_buffer == vec![OUT_ID_DUMP] {
            self.output_len =
                Some(unpack_u16_little([self.output_len.unwrap() as u8, byte]) as usize);
            println!("dump length: {:?}", self.output_len);
            self.long_output = true;
            return false;
        }

        if self.output_buffer.len() == self.output_len.unwrap()
            && byte == OUT_ID_END
            && self.output_buffer[0] < 32
        {
            println!("handling message...");
            self.handle_output().unwrap();
            self.clear();
            return true;
        }

        self.output_buffer.push(byte);
        println!("output buffer: {:?}", self.output_buffer);
        return false;
    }

    fn handle_output(&mut self) -> Result<(), Box<dyn Error>> {
        let output = Output::from_bytes(self.output_buffer.clone())?;
        match output.output_type {
            OutputType::MsgAck => {
                println!("Message acknowledged");
                return Ok(());
            }
            OutputType::MsgErr => {
                println!("Message error");
                return Ok(());
            }
            OutputType::Dump => {
                println!("Dump");
                return Ok(());
            }
            _ => {}
        }

        if output.output_id == self.next_output_id.wrapping_sub(1) {
            // This is a retransmission of the previous message.
            // acknowledge it and ignore it.
            println!("Retransmission of message {:?}", output.output_id);
            self.queue_input(Input::acknowledge(output.output_id))?;
            return Ok(());
        }
        if !output.validate_checksum() || output.output_id != self.next_output_id {
            println!("Message error: {:?}", output.data);
            self.queue_input(Input::msg_err(output.output_id))?;
            return Ok(());
        }

        // acknowledge the message
        println!("Message success: {:?}", output.data);
        self.queue_input(Input::acknowledge(output.output_id))?;
        self.next_output_id = self.next_output_id.wrapping_add(1);
        println!("Next output ID: {:?}", self.next_output_id);
        Ok(())
    }

    fn update_line_buffer(&mut self, byte: u8) {
        self.line_buffer.push(byte);
        if self.line_buffer.ends_with(&vec![13, 10]) && self.line_buffer[1] >= 32 {
            if let Ok(line) = std::str::from_utf8(&self.line_buffer) {
                if let Some(sender) = self.line_sender.as_ref() {
                    sender.send(line.to_string()).unwrap();
                }
                if self.print_lines {
                    print!("[Hub STDOUT] {}", line);
                }
                self.clear();
            }
        }
    }

    fn output_incomplete(&self) -> bool {
        self.output_len.is_some()
    }

    fn clear(&mut self) {
        self.line_buffer.clear();
        self.output_len = None;
        self.output_buffer.clear();
        self.long_output = false;
    }

    async fn input_queue_task(
        mut input_queue_receiver: mpsc::UnboundedReceiver<Input>,
        input_sender: UnboundedSender<Vec<u8>>,
        mut response_receiver: mpsc::UnboundedReceiver<Output>,
    ) {
        let mut next_input_id: u8 = 0;
        while let Some(input) = input_queue_receiver.recv().await {
            println!("Sending response: {:?}", input);
            let data = input.to_bytes(next_input_id);
            if input.expect_response() {
                loop {
                    input_sender.send(data.clone()).unwrap();
                    let maybe_response =
                        timeout(Duration::from_millis(500), response_receiver.recv()).await;
                    match maybe_response {
                        Ok(Some(response)) => match response.output_type {
                            OutputType::MsgAck => {
                                assert_eq!(response.data[0], next_input_id);
                                println!("Message acknowledged");
                                break;
                            }
                            OutputType::MsgErr => {
                                println!("Message error, retrying...");
                            }
                            _ => {
                                panic!("Unexpected response type");
                            }
                        },
                        Ok(None) => {
                            panic!("Response channel closed");
                        }
                        Err(_) => {
                            println!("Message timeout, retrying...");
                        }
                    }
                }
                next_input_id = next_input_id.wrapping_add(1);
            } else {
                input_sender.send(data).unwrap();
            }
        }
    }
}

pub struct IOHub {
    hub: Arc<Mutex<PybricksHub>>,
    io_state: Option<Arc<Mutex<IOState>>>,
}

impl IOHub {
    pub fn new(name: String) -> Self {
        IOHub {
            hub: Arc::new(Mutex::new(PybricksHub::new(name.into()))),
            io_state: None,
        }
    }

    pub async fn discover(&mut self, adapter: &BLEAdapter) -> Result<(), Box<dyn Error>> {
        let mut hub = self.hub.lock().await;
        hub.discover(adapter).await?;
        let output_receiver = hub.subscribe_output()?;

        let (input_sender, mut input_receiver) = mpsc::unbounded_channel();
        let io_state = Arc::new(Mutex::new(IOState::new(input_sender)));
        self.io_state = Some(io_state.clone());

        tokio::spawn(Self::forward_output_task(output_receiver, io_state));
        let hub = self.hub.clone();
        tokio::spawn(async move {
            while let Some(data) = input_receiver.recv().await {
                let unlocked_hub = hub.lock().await;
                unlocked_hub.write_stdin(&data).await.unwrap();
            }
        });
        Ok(())
    }

    pub async fn connect(&self) -> Result<(), Box<dyn Error>> {
        let mut hub = self.hub.lock().await;
        hub.connect().await?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().await;
        hub.disconnect().await?;
        Ok(())
    }

    pub async fn download_program(&self, name: &Path) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().await;
        hub.download_program(name).await?;
        Ok(())
    }

    pub async fn start_program(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().await;
        hub.start_program().await?;
        Ok(())
    }

    pub async fn stop_program(&self) -> Result<(), Box<dyn Error>> {
        let hub = self.hub.lock().await;
        hub.stop_program().await?;
        Ok(())
    }

    pub async fn queue_input(&self, input: Input) -> Result<(), Box<dyn Error>> {
        let io_state = self.io_state.as_ref().ok_or("Not connected")?.lock().await;
        io_state.queue_input(input)?;
        Ok(())
    }

    async fn forward_output_task(
        mut output_receiver: broadcast::Receiver<u8>,
        io_state: Arc<Mutex<IOState>>,
    ) {
        loop {
            let result = timeout(Duration::from_millis(300), output_receiver.recv()).await;
            let mut io_state = io_state.lock().await;
            match result {
                Ok(Ok(byte)) => {
                    io_state.on_output_byte_received(byte);
                }
                Ok(Err(_)) => {
                    println!("Output channel closed");
                    break;
                }
                Err(_) => {
                    if io_state.output_incomplete() {
                        println!("Output channel timed out");
                        io_state.clear();
                        io_state
                            .queue_input(Input::msg_err(io_state.next_input_id))
                            .unwrap();
                    }
                }
            }
        }
    }
}
