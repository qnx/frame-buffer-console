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

use crate::qnxmsg::transaction::Transaction;
use libc::{c_short, ino_t, off_t};

const DIRENT_SIZE: usize = std::mem::size_of::<dirent>();
const NAME_MAX: usize = 511;
const BUFFER_SIZE: usize = (DIRENT_SIZE + NAME_MAX) * 2;

/// Reply buffer for an _IO_READ message on a directory.
/// The buffer is filled with aligned dirent structures, providing information
/// on each node within the directory.
pub struct ReadDir {
    reply_max: usize,
    buffer: [u8; BUFFER_SIZE],
    buffer_pos: usize,
    offset: i64
}

/// Result values for ReadDir::add().
pub enum ReadDirAddResult {
    /// Successfully added
    Ok,
    /// Not added, buffer full and needs to be replied
    NeedFlush,
    /// Not added, reply length reached the value requested by the client
    Done
}

#[repr(C, packed)]
struct dirent {
    pub d_ino: ino_t,
    pub d_offset: off_t,
    pub d_reclen: c_short,
    pub d_namelen: c_short
}

const DIRENT_ALIGNMENT: usize = 8;

/// dirent structures are expected to be aligned, even though the length of each
/// structure may not be a multiple of the alignment size.
fn align_dirent(offset: usize) -> usize {
    (offset + DIRENT_ALIGNMENT - 1usize) & !(DIRENT_ALIGNMENT - 1usize)
}

impl ReadDir {
    /// Initialize a new reply buffer.
    pub fn new(trans: &Transaction) -> Self {
        Self {
            reply_max: trans.info.dstmsglen as usize,
            buffer: [0; BUFFER_SIZE],
            buffer_pos: 0,
            offset: 0
        }
    }

    /// Add a dirent structure for a single node to the reply.
    pub fn add(&mut self, ino: u64, name: &str) -> ReadDirAddResult  {
        // Calculate the lengths for the dirent structure.
        // It is assumed that the server keeps names to at most NAME_MAX.
        // Note that namelen does not include a terminating NUL, but d_name (and
        // therefore the record length) does.
        let namelen = name.len();
        assert!(namelen < NAME_MAX);
        let reclen = align_dirent(DIRENT_SIZE + namelen + 1);

        if (self.buffer_pos + reclen) > BUFFER_SIZE {
            return ReadDirAddResult::NeedFlush;
        }

        if (self.buffer_pos + reclen) > self.reply_max {
            return ReadDirAddResult::Done;
        }

        // Reinterpret the current buffer position as a dirent structure.
        let from = self.buffer_pos;
        let to = from + DIRENT_SIZE;
        let (head, ent, _) = unsafe {
            self.buffer[from..to].align_to_mut::<dirent>()
        };
        assert!(head.is_empty());

        ent[0].d_ino = ino;
        ent[0].d_offset = self.offset;
        ent[0].d_namelen = namelen as i16;
        ent[0].d_reclen = reclen as i16;

        // Write the name to the buffer, plus a terminating NUL.
        let from = to;
        let to = from + namelen;
        self.buffer[from..to].copy_from_slice(name.as_bytes());
        self.buffer[to] = 0u8;
        self.buffer_pos += reclen;
        ReadDirAddResult::Ok
    }

    /// Get access to the reply buffer for use with a call to
    /// Transaction::reply().
    pub fn get_buffer(&self) -> (&[u8], usize) {
        (&self.buffer, self.buffer_pos)
    }
}
