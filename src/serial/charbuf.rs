/// The size of the buffer.
/// In canonical mode, this is the maximum length of a line.
const CHAR_BUF_SIZE: usize = 1024;

/// Circular buffer of pending char events.
pub struct CharBuf {
    chars: [u8; CHAR_BUF_SIZE],
    head: usize,
    tail: usize,
    count: usize
}

impl CharBuf {
    pub fn new() -> Self {
        CharBuf { chars: [0; CHAR_BUF_SIZE], head: 0, tail: 0, count: 0 }
    }

    /// Add characters to the buffer.
    /// Characters are silently dropped if the buffer is full.
    pub fn push(&mut self, chars: &[u8]) {
        let room = CHAR_BUF_SIZE - self.count;
        let len = std::cmp::min(chars.len(), room);
        for chr in &chars[0..len] {
            self.chars[self.tail] = *chr;
            self.tail = (self.tail + 1) % CHAR_BUF_SIZE;
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
        if self.head + len > CHAR_BUF_SIZE {
            len = CHAR_BUF_SIZE - self.head;
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

        self.head = (self.head + count) % CHAR_BUF_SIZE;
        self.count -= count;
        pop_count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
