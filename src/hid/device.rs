/// Human interface device.

#[derive(PartialEq, Debug)]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Other
}

#[derive(Debug)]
pub struct Device {
    pub _device_number: u32,
}

impl Device {
    pub fn new(device_number: u32) -> Device {
        Device { _device_number: device_number }
    }
}
