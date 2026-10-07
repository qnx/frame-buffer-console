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

use crate::hid::*;
use crate::hid::key::*;

/// Input report parser
#[derive(Debug)]
pub struct InputReport {
    data: Vec<u8>
}

impl InputReport {
    pub fn new(event: &HidEvent, device: &Keyboard) -> Result<InputReport, ()> {
        // Extract the data.
        const HID_EVENT_SIZE: usize = size_of::<HidEvent>() - MAX_REPORT_LEN;
        let event_length = event.length as usize;
        if event_length < HID_EVENT_SIZE {
            println!("Bad input report");
            return Err(());
        }

        let data_offset = event_length - HID_EVENT_SIZE;

        let data_len = event.dlen as usize;
        if data_len != device.report_len as usize {
            println!("Bad input report");
            return Err(());
        }

        if data_len + data_offset > MAX_REPORT_LEN {
            println!("Bad input report");
            return Err(());
        }

        let report_data = &event.report[data_offset..data_offset + data_len];
        Ok(InputReport { data: report_data.to_vec() })
    }

    /// Convert an input report into a key.
    pub fn as_key(&self) -> Option<Key> {
        if self.data.len() < 8 {
            return None;
        }

        translate_key(&self.data[0..8])
    }
}

const HID_MOD_LSHIFT: u8 = 0x02;
const HID_MOD_RSHIFT: u8 = 0x20;
const HID_MOD_SHIFT: u8 = HID_MOD_LSHIFT | HID_MOD_RSHIFT;
const HID_MOD_LCTRL: u8 = 0x01;
const HID_MOD_RCTRL: u8 = 0x10;
const HID_MOD_CTRL: u8 = HID_MOD_LCTRL | HID_MOD_RCTRL;
const HID_MOD_LALT: u8 = 0x04;
const HID_MOD_RALT: u8 = 0x40;
const HID_MOD_ALT: u8 = HID_MOD_LALT | HID_MOD_RALT;

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

/// Implementation of key conversion.
fn translate_key(data: &[u8]) -> Option<Key> {
    // Translate modifiers.
    // This gets rid of the distinction between left and right versions, as well as shift (the
    // impact of shift is folded into the character).
    let mut modifier = 0;
    let is_shift = data[0] & HID_MOD_SHIFT != 0;

    if data[0] & HID_MOD_ALT != 0 {
        modifier |= MOD_ALT;
    }

    if data[0] & HID_MOD_CTRL != 0 {
        modifier |= MOD_CTRL;
    }

    let code = data[2] as usize;
    if code > KEY_USAGE_TABLE.len() {
        return None;
    }

    // Translate the usage code into the resulting character.
    let chr = if is_shift {
        KEY_USAGE_TABLE[code].1
    } else {
        KEY_USAGE_TABLE[code].0
    };

    if chr == 0 {
        return None;
    }

    // Add the special modifier, if needed.
    modifier |= KEY_USAGE_TABLE[code].2;
    Some (Key { chr, modifier })
}
