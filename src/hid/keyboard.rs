/// Keyboard device

use crate::hid::Key;

#[derive(Debug)]
pub struct Keyboard {
    pub device_number: u32,
    pub report_len: u8,
    pressed_key: Option<Key>
}

impl Keyboard {
    pub fn new(device_number: u32, report_len: u8) -> Keyboard {
        Keyboard { device_number, report_len, pressed_key: None }
    }

    pub fn key_pressed(&mut self, key: Option<Key>) -> Option<Key> {
        // Determine if there was a change.
        let prev_key = std::mem::replace(&mut self.pressed_key, key);
        let new_key = self.pressed_key.clone();

        let key =
            match prev_key {
                Some(pkey) => {
                    // There is a already a pressed key, check for a change.
                    match new_key {
                        Some(key) => {
                            if key != pkey {
                                Some(key)
                            } else {
                                None
                            }
                        }
                        None => None
                    }
                },
                None => {
                    // No key was pressed before.
                    match new_key {
                        Some(key) => Some(key),
                        None => None
                    }
                }
            };

        key
    }
}
