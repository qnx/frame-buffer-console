/// Input report parser

use crate::hid::*;

#[derive(Debug)]
pub struct InputReport {
    data: Vec<u8>
}

impl InputReport {
    pub fn new(event: &HidEvent, device: &Device) -> Result<InputReport, ()> {
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

const MOD_LSHIFT: u8 = 0x02;
const MOD_RSHIFT: u8 = 0x20;
const MOD_SHIFT: u8 = MOD_LSHIFT | MOD_RSHIFT;

/// Implementation of key conversion.
fn translate_key(data: &[u8]) -> Option<Key> {
    let mut modifier = data[0];
    let is_shift = (modifier & MOD_SHIFT) != 0;
    modifier &= !MOD_SHIFT;

    let key = data[2];

    let mut chr = 0;
    if key >= 0x4 && key < 0x1d {
        if is_shift {
            chr = key - 0x4 + b'A';
        } else {
            chr = key - 0x4 + b'a';
        }
    }

    if chr == 0 {
        return None;
    }

    Some (Key { chr, modifier })
}
