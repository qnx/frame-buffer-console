/// Module for communicating with io-hid

use libc;
use std::ptr::*;
use std::ffi::CStr;

pub const _IO_MSG: u16 = 0x113;

pub const _IOMGR_HID: u16 = 0x13;

pub const HID_VERSION: u16 = 0x100;
pub const HIDD_VERSION: u16 = 0x100;
pub const HIDDI_CMD_CONNECT: u16 = 0x101;

pub const HIDDI_FLAGS_EVTPOLL: u32 = 0x1;
pub const HIDDI_FLAGS_EXCL_ATTACH: u32 = 0x2;
pub const HIDDI_FLAGS_ENUMERATE: u32 = 0x4;

#[repr(C, packed)]
#[derive(Debug)]
pub struct IoMsg {
    msg_type: u16,
    combine_len: u16,
    pub mgrid: u16,
    pub subtype: u16
}

#[repr(C, packed)]
struct ConnectCmd {
    devno: u32,
    param: u32,
    flags: u32,
    vhid: u16,
    vhidd: u16,
    dlen: u16,
    reserved: [u8; 6],
    data: *const u8
}

/// A connection to the io-hid server.
pub struct Hid {
    fd: i32
}

fn msg_hid(fd: i32, hdr: &IoMsg, cmd: &ConnectCmd) -> Result<i64, i32> {
    let result = unsafe {
        let iovs = vec![
            libc::iovec {
                iov_base: addr_of!(*hdr) as *mut libc::c_void,
                iov_len: size_of::<IoMsg>()
            },
            libc::iovec {
                iov_base: addr_of!(*cmd) as *mut libc::c_void,
                iov_len: size_of::<ConnectCmd>()
            }
        ];

        libc::MsgSendv_r(fd, addr_of!(iovs[0..1]) as *const libc::iovec, 2,
                         std::ptr::null(), 0)
    };

    if result < 0 {
        return Err(-result as i32);
    }

    Ok(result)
}

fn open(path: &CStr, mode: libc::c_int) -> Result<i32, i32> {
    let fd = unsafe {
        libc::open(path.as_ptr(), mode)
    };

    if fd == -1 {
        return Err(unsafe { *libc::__get_errno_ptr() });
    }

    Ok(fd)
}

impl Hid {
    pub fn new() -> Result<Hid, i32> {
        // Open a connection to the server.
        let fd = open(c"/dev/io-hid/io-hid", libc::O_RDWR)?;

        // Send the connect message.
        let hdr = IoMsg {
            msg_type: _IO_MSG,
            subtype: HIDDI_CMD_CONNECT,
            mgrid: _IOMGR_HID,
            combine_len: size_of::<IoMsg>() as u16 + size_of::<ConnectCmd>() as u16
        };

        let cmd = ConnectCmd {
            devno: 0,
            param: 0,
            flags: HIDDI_FLAGS_EVTPOLL | HIDDI_FLAGS_EXCL_ATTACH | HIDDI_FLAGS_ENUMERATE,
            vhid: HID_VERSION,
            vhidd: HIDD_VERSION,
            dlen: 0,
            reserved: [0, 0, 0, 0, 0, 0],
            data: std::ptr::null()
        };

        msg_hid(fd, &hdr, &cmd)?;

        Ok(Hid { fd })
    }
}
