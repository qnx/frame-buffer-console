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

// FIXME:
// Add support for a line that exceeds the screen size with horizontal scrolling.

use crate::display::Display;
use crate::hid::key::*;

const LINE_SIZE: usize = 256;

/// Line editing in canonical mode.
pub struct LineEditor {
    /// Line buffer.
    buffer: [u8; LINE_SIZE],
    /// Line length (highest inserted index).
    len: usize,
    /// Insertion position.
    pos: usize,
    /// Screen row being edited.
    row: u32
}

impl LineEditor {
    pub fn new() -> Self {
        LineEditor { buffer: [0; LINE_SIZE], len: 0, pos: 0, row: 0 }
    }

    /// Start editing the given line.
    pub fn set_line(&mut self, row: u32) {
        self.row = row;
        self.pos = 0;
        self.len = 0;
    }

    /// Get the contents of the line buffer.
    pub fn get_line(&self) -> &[u8] {
        &self.buffer[0..self.len]
    }

    /// Handle a key event in canonical mode.
    /// Returns None if the key was fully consumed, or a key to process further by the caller.
    pub fn key_event(&mut self, key: Key, display: &mut Display) -> Option<Key> {
        if key.is_ctrl() {
            return self.handle_ctrl(key);
        }

        let chr = key.chr;
        match chr {
            b'\n' => return self.newline(display),
            0x08 => return self.backspace(display),
            _ => {}
        };

        // Drop the key if the buffer is full.
        // Note that one position is left for a (potential) newline.
        if self.len == LINE_SIZE - 1 {
            return None;
        }

        // Insert the key into the line buffer.
        if self.pos == self.len {
            // Append to the end of the line.
            self.len += 1;
        } else {
            // Insert in the middle of the line.
            for idx in self.len - 1..self.pos {
                self.buffer[idx] = self.buffer[idx - 1];
            }
        }
        self.buffer[self.pos] = chr;

        // Update the display.
        display.insert_line(&self.buffer[self.pos..self.len], self.row, self.pos as u32,
                            (self.pos + 1) as u32);

        self.pos += 1;

        None
    }

    fn handle_ctrl(&mut self, key: Key) -> Option<Key> {
        match key.chr {
            b'a' => {
                // Ctrl-A, go to beginning of line
                self.pos = 0;
                Some(key)
            }
            b'd' => {
                // Ctrl-D, end of transimission.
                Some(Key { chr: KEY_CANON_EOT, modifier: MOD_SPECIAL })
            }
            _ => Some(key)
        }
    }

    /// Newline terminates editing of the current line.
    fn newline(&mut self, display: &mut Display) -> Option<Key> {
        // The newline character is always added to the end of the line.
        assert!(self.len < LINE_SIZE);
        self.buffer[self.len] = b'\n';
        self.len += 1;

        // Update the display by showing the complete line.
        display.insert_line(&self.buffer[0..self.len], self.row, 0, 0);

        // Let the caller know that processing is done for this line.
        Some(Key { chr: KEY_CANON_EOL, modifier: MOD_SPECIAL })
    }

    /// Backspace erases one character back.
    fn backspace(&mut self, display: &mut Display) -> Option<Key> {
        // Shift characters back.
        if self.pos != 0 {
            for idx in self.pos..self.len {
                self.buffer[idx - 1] = self.buffer[idx];
            }
            self.buffer[self.len - 1] = b' ';
            self.pos -= 1;

            // Update the display.
            // The slice includes the space that was added at the formerly-last position.
            display.insert_line(&self.buffer[self.pos..self.len], self.row, self.pos as u32,
                                self.pos as u32);
            self.len -= 1;
        }

        None
    }
}
