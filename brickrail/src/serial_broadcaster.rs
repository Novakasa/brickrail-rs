use std::time::Duration;

use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use serialport::SerialPortType;

pub struct SerialBroadcasterPlugin;

#[derive(Component)]
pub struct Broadcaster {
    device: String,
    led_on: bool,
}

impl Plugin for SerialBroadcasterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, connect_serial);
        app.add_systems(
            Update,
            broadcast_serial.run_if(input_just_pressed(KeyCode::KeyB)),
        );
    }
}
fn find_pico_port() -> Option<String> {
    serialport::available_ports()
        .ok()?
        .into_iter()
        .find_map(|p| {
            if let SerialPortType::UsbPort(info) = &p.port_type {
                // MicroPython on Pico typically uses VID 2E8A
                if info.vid == 0x2E8A {
                    return Some(p.port_name);
                }
            }
            None
        })
}

fn connect_serial(mut commands: Commands) {
    let port_name = find_pico_port();
    println!("Found Pico port: {:?}", port_name);
    commands.spawn(Broadcaster {
        device: port_name.clone().unwrap_or_else(|| "Not found".to_string()),
        led_on: false,
    });
}

fn broadcast_serial(mut broadcaster_query: Single<&mut Broadcaster>) {
    let mut port = serialport::new(&broadcaster_query.device, 115200)
        .timeout(Duration::from_secs(2))
        .open()
        .unwrap();
    match broadcaster_query.led_on {
        true => port.write(b"off\n").unwrap(),
        false => port.write(b"on\n").unwrap(),
    };
    broadcaster_query.led_on = !broadcaster_query.led_on;
}
