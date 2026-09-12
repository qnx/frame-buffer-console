/// Human interface device.

#[derive(PartialEq, Debug)]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Other
}

#[derive(Debug)]
pub struct Device {
    pub device_number: u32,
    pub report_len: u8
}

impl Device {
    pub fn new(device_number: u32, report_len: u8) -> Device {
        Device { device_number, report_len }
    }
}
