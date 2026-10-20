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

use std::ptr::*;

pub const CONNECT: u16 = 0x101;
pub const POLL: u16 = 0x102;
pub const REPORT_ATTACH: u16 = 0x103;
pub const SET_IDLE: u16 = 0x10d;
pub const REPORT_DESC: u16 = 0x114;

#[repr(C, packed)]
#[derive(Debug)]
struct IoMsg {
    msg_type: u16,
    combine_len: u16,
    mgrid: u16,
    subtype: u16
}

#[repr(C, packed)]
pub struct ConnectCmd {
    pub devno: u32,
    pub param: u32,
    pub flags: u32,
    pub vhid: u16,
    pub vhidd: u16,
    pub _pad: [u64; 2]
}

#[repr(C, packed)]
pub struct PollCmd {
    pub devno: u32,
    pub param: u32,
    pub buffer: u32,
    pub _pad1: u32,
    pub _pad2: [u64; 2]
}

#[repr(C, packed)]
pub struct ReportCmd {
    pub devno: u32,
    pub param: u32,
    pub _pad: [u64; 3]
}

#[repr(C, packed)]
pub struct AttachCmd {
    pub devno: u32,
    pub param: u32,
    pub flags: u32,
    pub report_type: u16,
    pub report_id: u8,
    pub _pad1: u8,
    pub _pad2: [u64; 2]
}

#[repr(C, packed)]
pub struct IdleCmd {
    pub devno: u32,
    pub param: u32,
    pub report_id: u8,
    pub _pad1: u8,
    pub idle_rate: u16,
    pub _pad2: u32,
    pub _pad3: [u64; 2]
}

const _IO_MSG: u16 = 0x113;
const _IOMGR_HID: u16 = 0x13;

/// Send a command to the io-hid server.
/// The command comprises a _IO_MSG header and a command structure.
pub fn send<T, U>(fd: i32, cmd_id: u16, cmd: &T, reply: Option<&mut U>) -> Result<i64, i32> {
    let hdr = IoMsg {
        msg_type: _IO_MSG,
        subtype: cmd_id,
        mgrid: _IOMGR_HID,
        combine_len: size_of::<IoMsg>() as u16 + size_of::<T>() as u16
    };

    let result = unsafe {
        // Construct two-part message.
        // FIXME:
        // Must use addr_of!() because libc::iovec does not provide iov_base_const.
        let siovs = vec![
            libc::iovec {
                iov_base: addr_of!(hdr) as *mut libc::c_void,
                iov_len: size_of::<IoMsg>()
            },
            libc::iovec {
                iov_base: addr_of!(*cmd) as *mut libc::c_void,
                iov_len: size_of::<T>()
            }
        ];

        // Construct one-part reply buffer.
        let (reply_addr, reply_len) = match reply {
            Some(u) => (u as *mut U as *mut libc::c_void,
                        size_of::<U>()),
            None => (std::ptr::null_mut(), 0)
        };

        let riov = libc::iovec { iov_base: reply_addr, iov_len: reply_len };

        // Send the message.
        libc::MsgSendv_r(fd, addr_of!(siovs[0..1]) as *const libc::iovec, 2,
                         addr_of!(riov) as *const libc::iovec, 1)
    };

    if result < 0 {
        return Err(-result as i32);
    }

    Ok(result)
}
