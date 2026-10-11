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

/// Implementation of key conversion.
fn translate_key(data: &[u8]) -> Option<Key> {
    // Translate modifiers.
    let mut modifier = 0;

    if data[0] & HID_MOD_SHIFT != 0 {
        modifier |= MOD_SHIFT;
    }

    if data[0] & HID_MOD_ALT != 0 {
        modifier |= MOD_ALT;
    }

    if data[0] & HID_MOD_CTRL != 0 {
        modifier |= MOD_CTRL;
    }

    Key::new(data[2], modifier)
}
