const KEY_BUF_SIZE: usize = 64;

/// Circular buffer of pending key events.
pub struct KeyBuf {
    keys: [u8; KEY_BUF_SIZE],
    head: usize,
    tail: usize,
    count: usize
}

impl KeyBuf {
    pub fn new() -> Self {
        KeyBuf { keys: [0; KEY_BUF_SIZE], head: 0, tail: 0, count: 0 }
    }

    /// Add a key to the buffer.
    /// The key is silently dropped if the buffer is full.
    pub fn push(&mut self, key: u8) {
        if self.count == KEY_BUF_SIZE {
            return;
        }

        self.keys[self.tail] = key;
        self.tail = (self.tail + 1) % KEY_BUF_SIZE;
        self.count += 1;
    }

    /// Get a view into the circular buffer.
    /// Used to reply to a client's read() request.
    /// The view contains the minimum of the requested size, the number of keys in the buffer, and
    /// the size from the head to the end of the buffer.
    pub fn get_data(&self, maxlen: usize) -> (&[u8], usize) {
        let mut len = maxlen;
        if len > self.count {
            len = self.count;
        }
        if self.head + len > KEY_BUF_SIZE {
            len = KEY_BUF_SIZE - self.head;
        }

        (&self.keys[self.head..self.head + len], len)
    }

    /// Remove elements from the queue.
    pub fn pop(&mut self, count: usize) -> usize {
        let pop_count = if count > self.count {
            self.count
        } else {
            count
        };

        self.head = (self.head + count) % KEY_BUF_SIZE;
        self.count -= count;
        pop_count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
