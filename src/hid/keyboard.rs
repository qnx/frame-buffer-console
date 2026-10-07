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

use crate::hid::key::Key;

/// Keyboard device
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
