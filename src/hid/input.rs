/// Input report parser

use crate::hid::*;
use crate::hid::key::*;

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

const KEY_USAGE_TABLE: [(u8, u8); 57] = [
    (0, 0),
    (0, 0),
    (0, 0),
    (0, 0),
    (b'a', b'A'),
    (b'b', b'B'),
    (b'c', b'C'),
    (b'd', b'D'),
    (b'e', b'E'),
    (b'f', b'F'),
    (b'g', b'G'),
    (b'h', b'H'),
    (b'i', b'I'),
    (b'j', b'J'),
    (b'k', b'K'),
    (b'l', b'L'),
    (b'm', b'M'),
    (b'n', b'N'),
    (b'o', b'O'),
    (b'p', b'P'),
    (b'q', b'Q'),
    (b'r', b'R'),
    (b's', b'S'),
    (b't', b'T'),
    (b'u', b'U'),
    (b'v', b'V'),
    (b'w', b'W'),
    (b'x', b'X'),
    (b'y', b'Y'),
    (b'z', b'Z'),
    (b'1', b'!'),
    (b'2', b'@'),
    (b'3', b'#'),
    (b'4', b'$'),
    (b'5', b'%'),
    (b'6', b'^'),
    (b'7', b'&'),
    (b'8', b'*'),
    (b'9', b'('),
    (b'0', b')'),
    (b'\n', b'\n'), // FIXME: Shift+Enter?
    (0x1b, 0x1b),
    (0x08, 0x08),
    (b'\t', b'\t'),
    (b' ', b' '),
    (b'-', b'_'),
    (b'=', b'+'),
    (b'[', b'{'),
    (b']', b'}'),
    (b'\\', b'|'),
    (0, 0), // FIXME: Non-US kyeboard # and ˜?
    (b';', b':'),
    (b'\'', b'\"'),
    (b'`', b'~'),
    (b',', b'<'),
    (b'.', b'>'),
    (b'/', b'?'),
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

    Some (Key { chr, modifier })
}
