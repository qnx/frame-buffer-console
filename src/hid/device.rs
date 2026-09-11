/// Human interface device.

#[derive(PartialEq)]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Other
}

pub struct Device {
    pub device_number: u32,
    device_type: DeviceType
}

impl Device {
    pub fn new(device_number: u32) -> Device {
        Device { device_number, device_type: DeviceType::Other }
    }

    pub fn set_type(&mut self, device_type: DeviceType) {
        self.device_type = device_type;
    }

    pub fn is_keyboard(&self) -> bool {
        self.device_type == DeviceType::Keyboard
    }
}
