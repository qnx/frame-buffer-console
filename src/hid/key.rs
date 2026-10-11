// Copyright (c) 2026 BlackBerry Limited.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::qnxmsg::Pulse;

pub const MOD_SHIFT: u8 = 0x01;
pub const MOD_CTRL: u8 = 0x02;
pub const MOD_ALT: u8 = 0x04;
pub const MOD_SPECIAL: u8 = 0x08;

pub const KEY_LEFT_ARROW: u8 = 1;
pub const KEY_RIGHT_ARROW: u8 = 2;
pub const KEY_UP_ARROW: u8 = 3;
pub const KEY_DOWN_ARROW: u8 = 4;
pub const KEY_CANON_EOL: u8 = 5;
pub const KEY_CANON_EOT: u8 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Key {
    pub code: u8,
    pub modifier: u8
}

const KEY_USAGE_TABLE: [(u8, u8, u8); 83] = [
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (b'a', b'A', 0),
    (b'b', b'B', 0),
    (b'c', b'C', 0),
    (b'd', b'D', 0),
    (b'e', b'E', 0),
    (b'f', b'F', 0),
    (b'g', b'G', 0),
    (b'h', b'H', 0),
    (b'i', b'I', 0),
    (b'j', b'J', 0),
    (b'k', b'K', 0),
    (b'l', b'L', 0),
    (b'm', b'M', 0),
    (b'n', b'N', 0),
    (b'o', b'O', 0),
    (b'p', b'P', 0),
    (b'q', b'Q', 0),
    (b'r', b'R', 0),
    (b's', b'S', 0),
    (b't', b'T', 0),
    (b'u', b'U', 0),
    (b'v', b'V', 0),
    (b'w', b'W', 0),
    (b'x', b'X', 0),
    (b'y', b'Y', 0),
    (b'z', b'Z', 0),
    (b'1', b'!', 0),
    (b'2', b'@', 0),
    (b'3', b'#', 0),
    (b'4', b'$', 0),
    (b'5', b'%', 0),
    (b'6', b'^', 0),
    (b'7', b'&', 0),
    (b'8', b'*', 0),
    (b'9', b'(', 0),
    (b'0', b')', 0),
    (b'\n', b'\n', 0), // FIXME: Shift+Enter?
    (0x1b, 0x1b, 0),
    (0x08, 0x08, 0),
    (b'\t', b'\t', 0),
    (b' ', b' ', 0),
    (b'-', b'_', 0),
    (b'=', b'+', 0),
    (b'[', b'{', 0),
    (b']', b'}', 0),
    (b'\\', b'|', 0),
    (0, 0, 0), // FIXME: Non-US kyeboard # and ˜?
    (b';', b':', 0),
    (b'\'', b'\"', 0),
    (b'`', b'~', 0),
    (b',', b'<', 0),
    (b'.', b'>', 0),
    (b'/', b'?', 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (KEY_RIGHT_ARROW, 0, MOD_SPECIAL),
    (KEY_LEFT_ARROW, 0, MOD_SPECIAL),
    (KEY_DOWN_ARROW, 0, MOD_SPECIAL),
    (KEY_UP_ARROW, 0, MOD_SPECIAL),
    ];

impl Key {
    pub fn new(code: u8, modifier: u8) -> Option<Key> {
        // Verify that the code represents a valid key.
        let idx = code as usize;
        if idx > KEY_USAGE_TABLE.len() {
            return None;
        }

        if KEY_USAGE_TABLE[idx].0 == 0 {
            return None;
        }

        // Handle special keys.
        let (code, modifier) = if KEY_USAGE_TABLE[idx].2 == MOD_SPECIAL {
            (KEY_USAGE_TABLE[idx].0, modifier | MOD_SPECIAL)
        } else {
            (code, modifier)
        };

        Some(Key{ code, modifier })
    }

    pub fn from_pulse(pulse: &Pulse) -> Self {
        let value = pulse.value.sival_ptr as u64;
        Key { code: (value & 0xff) as u8, modifier: ((value >> 8) & 0xff) as u8 }
    }

    pub fn to_pulse_value(&self) -> i64 {
        let value: i64 = self.code as i64 | (self.modifier as i64) << 8;
        value
    }

    pub fn is_ctrl(&self) -> bool {
        (self.modifier & MOD_CTRL) != 0
    }

    pub fn is_special(&self) -> bool {
        (self.modifier & MOD_SPECIAL) != 0
    }

    pub fn as_char(&self) -> u8 {
        if (self.modifier & MOD_SHIFT) == 0 {
            KEY_USAGE_TABLE[self.code as usize].0
        } else {
            KEY_USAGE_TABLE[self.code as usize].1
        }
    }
}
