use crate::qnxmsg::Pulse;

pub const MOD_CTRL: u8 = 0x01;
pub const MOD_ALT: u8 = 0x02;
pub const MOD_SPECIAL: u8 = 0x04;

pub const KEY_LEFT_ARROW: u8 = 1;
pub const KEY_RIGHT_ARROW: u8 = 2;
pub const KEY_UP_ARROW: u8 = 3;
pub const KEY_DOWN_ARROW: u8 = 4;

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

    pub fn is_special(&self) -> bool {
        (self.modifier & MOD_SPECIAL) != 0
    }
}
