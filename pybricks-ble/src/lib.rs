pub mod io_hub;
pub mod pybricks_hub;

fn unpack_u32_little(data: Vec<u8>) -> u32 {
    (data[0] as u32) | ((data[1] as u32) << 8) | ((data[2] as u32) << 16) | ((data[3] as u32) << 24)
}

fn unpack_u16_little(data: [u8; 2]) -> u16 {
    (data[0] as u16) | ((data[1] as u16) << 8)
}

fn unpack_u16_big(data: [u8; 2]) -> u16 {
    (data[0] as u16) << 8 | (data[1] as u16)
}
