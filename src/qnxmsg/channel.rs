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

use crate::qnxmsg::MsgInfo;
use crate::qnxmsg::Pulse;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::libc_missing;

/// Channel object.
#[derive(Clone,Copy)]
pub struct Channel {
    pub chid: i32
}

impl Channel {
    /// Create a new channel.
    /// See the QNX documentation for ChannelCreate() for details.
    pub fn new(flags: u32) -> Result<Self, i32> {
        let chid =
            unsafe {
                libc::ChannelCreate_r(flags)
            };

        if chid < 0 {
            return Err(-chid);
        }

        let channel = Channel {
            chid
        };

        Ok(channel)
    }

    /// Receive a message or a pulse on the channel.
    /// Data is copied into the given message buffer, which can then be interpreted according to
    /// the type of message/pulse.
    /// A successful result holds the receive ID that can be used in other calls, such as
    /// MsgRead(), MsgReply(), etc.
    pub fn receive(&self, buf: &mut MsgBuf, info: &mut MsgInfo) -> Result<i64, i32> {
        let rcvid = unsafe {
            let buf_ptr = buf.as_void_ptr();
            let info_ptr = std::ptr::addr_of!(*info)
                as *mut libc::_msg_info64;

            libc_missing::MsgReceive_r(self.chid, buf_ptr,
                                       buf.get_size(),
                                       info_ptr)
        };

        if rcvid < 0 {
            return Err(-rcvid as i32);
        }

        if rcvid == 0 {
            buf.set_length(size_of::<Pulse>());
        } else {
            buf.set_length(info.msglen as usize);
        }
        Ok(rcvid)
    }

    /// Send a pulse on this channel.
    pub fn send_pulse(&self, priority: i32, code: i8, value: i64) -> Result<(), i32> {
        let chid = self.chid;
        let err = unsafe {
            libc_missing::MsgSendPulse_r(-chid, priority, code as i32, value)
        };

        if err != 0 {
            return Err(err);
        }

        Ok(())
    }
}
