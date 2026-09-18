/// Message buffer.
/// Used as a buffer when receiving a message. Should be decoded and transmuted
/// into a concrete message type.
#[derive(Debug)]
pub struct MsgBuf {
    data: [u8; 2048],
    len: usize
}

impl MsgBuf {
    pub fn new() -> Self {
        MsgBuf {
            data: [ 0u8; 2048 ],
            len: 0
        }
    }

    /// Get the first 16 bytes of the message, which represent its type.
    pub fn get_type(&self) -> u16 {
        if self.len < 2 {
            // No one should be sending a message of size smaller than 2, but
            // if they do we should handle it.
            return 0;
        }

        u16::from_le_bytes(self.data[0..2].try_into().unwrap())
    }

    pub fn set_length(&mut self, len: usize) {
        self.len = len;
    }

    pub fn get_length(&self) -> usize {
        self.len
    }

    pub fn valid_range(&self, from: usize, to: usize) -> bool {
        (from < to) && (to <= self.len)
    }

    /// Obtain a view into the message's payload.
    pub fn get_data(&self, from: usize, to: usize) -> Result<&[u8], i32> {
        if to > self.len {
            return Err(libc::ERANGE);
        }

        Ok(&self.data[from..to])
    }

    /// Obtain a view into the message's payload as a specific type.
    /// The offset argument specifies where in the payload the view starts.
    /// The end is determined by the size of the destination type.
    pub fn get_data_as<T>(&self, offset: usize) -> Result<&T, i32> {
        let len: usize = std::mem::size_of::<T>();
        let end = offset + len;
        if end > self.len {
            return Err(libc::ERANGE);
        }

        let (head, data, _) = unsafe { self.data[offset..end].align_to::<T>() };
        assert!(head.is_empty());
        Ok(&data[0])
    }

    /// Obtain a view into the message's payload as a slice of elements of a specific type.
    /// The offset argument specifies where in the payload the view starts.
    /// The end is determined by the size of the destination type.
    pub fn get_data_as_slice_of<T>(&self, offset: usize) -> Result<&[T], i32> {
        let (head, data, _) = unsafe { self.data[offset..].align_to::<T>() };
        assert!(head.is_empty());
        Ok(data)
    }

    /// Get access to the buffer for use with MsgReceive().
    pub fn as_void_ptr(&mut self) -> *mut libc::c_void {
        std::ptr::addr_of!(self.data) as *mut libc::c_void
    }

    /// Move data down by `off`.
    /// Used for handling combine messages.
    pub fn shift(&mut self, off: usize) {
        assert!(off < self.len);
        self.data.copy_within(off..self.len, 0);
        self.len -= off;
    }
}
