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
use crate::display::cursor::CursorDir;

/// Line editing in canonical mode.
pub struct LineEditor {
    /// Line buffer.
    buffer: Vec<u8>,
    /// Line length (highest inserted index).
    len: usize,
    /// Insertion position.
    pos: usize,
    /// Whether echo is on.
    echo: bool
}

impl LineEditor {
    pub fn new(columns: u32) -> Self {
        let mut buffer = Vec::<u8>::with_capacity(columns as usize);
        buffer.resize(columns as usize, 0);
        LineEditor { buffer, len: 0, pos: 0, echo: true }
    }

    /// Start editing the given line.
    pub fn reset_line(&mut self) {
        self.pos = 0;
        self.len = 0;
    }

    /// Get the contents of the line buffer.
    pub fn get_line(&self) -> &[u8] {
        &self.buffer[0..self.len]
    }

    /// Toggle echo mode.
    pub fn set_echo(&mut self, echo: bool) {
        self.echo = echo;
    }

    /// Handle a key event in canonical mode.
    /// Returns None if the key was fully consumed, or a key to process further by the caller.
    pub fn key_event(&mut self, key: Key, display: &mut Display) -> Option<Key> {
        if key.is_ctrl() {
            return self.handle_ctrl(key, display);
        }

        if key.is_special() {
            return self.handle_special(key, display);
        }

        let chr = key.as_char();
        match chr {
            b'\n' => return self.newline(display),
            0x08 => return self.backspace(display),
            _ => {}
        };

        // Drop the key if the buffer is full.
        // Note that one position is left for a (potential) newline.
        if self.len == self.buffer.len() - 1 {
            return None;
        }

        if self.pos != self.len {
            // Shift characters if not appending to the end of the line.
            for idx in (self.pos..self.len).rev() {
                self.buffer[idx + 1] = self.buffer[idx];
            }
        }

        // Insert the key into the line buffer.
        self.buffer[self.pos] = chr;
        self.len += 1;

        // Update the display.
        if self.echo {
            display.insert_at_cursor(&self.buffer[self.pos..self.len], Some(1));
        }

        self.pos += 1;

        None
    }

    fn handle_ctrl(&mut self, key: Key, display: &mut Display) -> Option<Key> {
        match key.as_char() {
            b'a' => {
                // Ctrl-A, go to beginning of line
                self.pos = 0;
                if self.echo {
                    display.move_in_line(CursorDir::BOL);
                }
                None
            }
            b'd' => {
                // Ctrl-D, end of transimission.
                Some(Key { code: KEY_CANON_EOT, modifier: MOD_SPECIAL })
            }
            b'e' => {
                // Ctrl-E, go to end of line
                if self.echo {
                    display.move_in_line(CursorDir::Forward((self.len - self.pos) as u32));
                }
                self.pos = self.len;
                None
            }
            _ => Some(key)
        }
    }

    /// Handle a key marked with the special modifier.
    fn handle_special(&mut self, key: Key, display: &mut Display) -> Option<Key> {
        match key.code {
            KEY_LEFT_ARROW => {
                if self.pos > 0 {
                    self.pos -= 1;
                    if self.echo {
                        display.move_in_line(CursorDir::Back(1));
                    }
                }
            }
            KEY_RIGHT_ARROW => {
                if self.pos < self.len {
                    self.pos += 1;
                    if self.echo {
                        display.move_in_line(CursorDir::Forward(1));
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Newline terminates editing of the current line.
    fn newline(&mut self, display: &mut Display) -> Option<Key> {
        // The newline character is always added to the end of the line.
        assert!(self.len < self.buffer.len());
        self.buffer[self.len] = b'\n';

        // Add a newline to the screen.
        if self.echo {
            let lf = [b'\n'];
            display.insert_at_cursor(&lf, None);
        }
        self.len += 1;

        // Let the caller know that processing is done for this line.
        Some(Key { code: KEY_CANON_EOL, modifier: MOD_SPECIAL })
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
            if self.echo {
                display.move_in_line(CursorDir::Back(1));
                display.insert_at_cursor(&self.buffer[self.pos..self.len], Some(0));
            }

            self.len -= 1;
        }

        None
    }
}
