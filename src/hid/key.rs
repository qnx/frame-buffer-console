use crate::qnxmsg::Pulse;

pub const MOD_LSHIFT: u8 = 0x02;
pub const MOD_RSHIFT: u8 = 0x20;
pub const MOD_SHIFT: u8 = MOD_LSHIFT | MOD_RSHIFT;
pub const MOD_LCTRL: u8 = 0x01;
pub const MOD_RCTRL: u8 = 0x10;
pub const MOD_CTRL: u8 = MOD_LCTRL | MOD_RCTRL;
pub const MOD_LALT: u8 = 0x04;
pub const MOD_RALT: u8 = 0x40;
pub const MOD_ALT: u8 = MOD_LALT | MOD_RALT;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Key {
    pub chr: u8,
    pub modifier: u8
}

impl Key {
    pub fn from_pulse(pulse: &Pulse) -> Self {
        let value = pulse.value.sival_ptr as u64;
        Key { chr: (value & 0xff) as u8, modifier: ((value >> 8) & 0xff) as u8 }
    }

    pub fn to_pulse_value(&self) -> i64 {
        let value: i64 = self.chr as i64 | (self.modifier as i64) << 8;
        value
    }

    pub fn is_ctrl(&self) -> bool {
        (self.modifier & MOD_CTRL) != 0
    }
}
