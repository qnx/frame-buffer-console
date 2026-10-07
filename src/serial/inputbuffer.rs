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

/// The size of the buffer.
const INPUT_BUFFER_SIZE: usize = 1024;

/// Circular buffer of pending input characters.
pub struct InputBuffer {
    chars: [u8; INPUT_BUFFER_SIZE],
    head: usize,
    tail: usize,
    count: usize
}

impl InputBuffer {
    pub fn new() -> Self {
        InputBuffer { chars: [0; INPUT_BUFFER_SIZE], head: 0, tail: 0, count: 0 }
    }

    /// Add characters to the buffer.
    /// Characters are silently dropped if the buffer is full.
    pub fn push(&mut self, chars: &[u8]) {
        let room = INPUT_BUFFER_SIZE - self.count;
        let len = std::cmp::min(chars.len(), room);
        for chr in &chars[0..len] {
            self.chars[self.tail] = *chr;
            self.tail = (self.tail + 1) % INPUT_BUFFER_SIZE;
            self.count += 1;
        }
    }

    /// Get a view into the circular buffer.
    /// Used to reply to a client's read() request.
    /// The view contains the minimum of the requested size, the number of chars in the buffer, and
    /// the size from the head to the end of the buffer.
    pub fn get_data(&self, maxlen: usize) -> (&[u8], usize) {
        let mut len = maxlen;
        if len > self.count {
            len = self.count;
        }
        if self.head + len > INPUT_BUFFER_SIZE {
            len = INPUT_BUFFER_SIZE - self.head;
        }

        (&self.chars[self.head..self.head + len], len)
    }

    /// Remove elements from the queue.
    pub fn pop(&mut self, count: usize) -> usize {
        let pop_count = if count > self.count {
            self.count
        } else {
            count
        };

        self.head = (self.head + count) % INPUT_BUFFER_SIZE;
        self.count -= count;
        pop_count
    }

    /// Remove all elements from the queue.
    pub fn flush(&mut self) {
        self.pop(self.count);
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
