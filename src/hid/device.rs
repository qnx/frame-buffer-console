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
}

impl Device {
    pub fn new(device_number: u32) -> Device {
        Device { device_number: device_number }
    }
}
