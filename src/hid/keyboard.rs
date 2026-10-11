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

use std::time::Instant;
use crate::hid::key::Key;

/// Keyboard device
#[derive(Debug)]
pub struct Keyboard {
    /// HID device identifier.
    pub device_number: u32,
    /// Number of bytes in the HID report for this device.
    pub report_len: u8,
    /// Last pressed key, or None.
    pressed_key: Option<Key>,
    /// Time in which the key press was last observed.
    key_time: Instant,
    /// Whether the key press is being repeated.
    repeat: bool
}

const KEY_FIRST_REPEAT: u128 = 200;
const KEY_NEXT_REPEAT: u128 = 60;

impl Keyboard {
    pub fn new(device_number: u32, report_len: u8) -> Keyboard {
        Keyboard {
            device_number,
            report_len,
            pressed_key: None,
            key_time: Instant::now(),
            repeat: false
        }
    }

    /// Handle an input event for the keyboard.
    /// Determines if a key event should be propagated based on the last key (if any) seen on this
    /// device.
    pub fn key_pressed(&mut self, key: Option<Key>) -> Option<Key> {
        if key.is_none() {
            // No key (including a key release).
            self.pressed_key = None;
            return None;
        }

        let is_changed = match self.pressed_key {
            None => true,
            Some(k) => {
                // For the purpose of repeated key events, consider a modified character and an
                // unmodified character to be the same.
                k.code != key.unwrap().code
            }
        };

        if is_changed {
            // Key changed, always emit an event.
            self.pressed_key = key;
            self.key_time = Instant::now();
            self.repeat = false;
            return key;
        }

        // If the key has not been repeated yet, declare a repeat after 100ms
        // Otherwise, a repeat occurs after 30ms.
        let repeat_time = if self.repeat {
            KEY_NEXT_REPEAT
        } else {
            KEY_FIRST_REPEAT
        };

        // No change, emit an event only if enough time has passed since the last event.
        if self.key_time.elapsed().as_millis() < repeat_time {
            return None;
        }

        self.key_time = Instant::now();
        self.repeat = true;
        key
    }
}
