/// Module for communicating with io-hid

use libc;
use std::ptr::*;
use std::ffi::CStr;
use std::thread::{self, JoinHandle};

pub const _IO_MSG: u16 = 0x113;

pub const _IOMGR_HID: u16 = 0x13;

pub const HID_VERSION: u16 = 0x100;
pub const HIDD_VERSION: u16 = 0x100;

pub const HIDDI_CMD_CONNECT: u16 = 0x101;
pub const HIDDI_CMD_POLL: u16 = 0x102;

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

#[repr(C, packed)]
struct PollCmd {
    devno: u32,
    param: u32,
    buffer: u32,
    pad: u32,
    dlen: u16,
    reserved: [u8; 6],
    data: *const u8
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct HidDeviceIdent {
    vendor_id: u32,
    product_id: u32,
    version: u32
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct HidDeviceInstance {
    _next: *const libc::c_void,
    _prev: *const libc::c_void,
    devno: u32,
    device_ident: HidDeviceIdent,
    _report_info: *const libc::c_void,
    report_desc_len: u16,
    reserved: [u8; 6]
}

#[repr(C, packed)]
union HidEventPerType {
    insertion: HidDeviceInstance,
    removal: HidDeviceInstance,
}

#[repr(C, packed)]
struct HidEvent {
    event_type: u16,
    pad1: u32,
    length: u16,
    event: HidEventPerType,
    reserved: [u8; 2],
    dlen: u16,
    pad2: u32,
    data: *const u8
}

/// A connection to the io-hid server.
pub struct Hid {
    fd: i32,
    thandle: Option<JoinHandle<()>>
}

/// Send a command to the io-hid server.
/// The command comprises a _IO_MSG header and a command structure.
fn send_cmd<T>(fd: i32, cmd_id: u16, cmd: &T) -> Result<i64, i32> {
    let hdr = IoMsg {
        msg_type: _IO_MSG,
        subtype: cmd_id,
        mgrid: _IOMGR_HID,
        combine_len: size_of::<IoMsg>() as u16 + size_of::<PollCmd>() as u16
    };

    let result = unsafe {
        let iovs = vec![
            libc::iovec {
                iov_base: addr_of!(hdr) as *mut libc::c_void,
                iov_len: size_of::<IoMsg>()
            },
            libc::iovec {
                iov_base: addr_of!(*cmd) as *mut libc::c_void,
                iov_len: size_of::<T>()
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

/// Safe wrapper for libc::open()
fn open(path: &CStr, mode: libc::c_int) -> Result<i32, i32> {
    let fd = unsafe {
        libc::open(path.as_ptr(), mode)
    };

    if fd == -1 {
        return Err(unsafe { *libc::__get_errno_ptr() });
    }

    Ok(fd)
}

/// Thread used to poll io-hid for events.
fn poll_thread(fd: i32) {
    // Construct a poll command.
    let cmd = PollCmd {
        devno: 0,
        param: 0,
        buffer: size_of::<HidEvent>() as u32,
        pad: 0,
        dlen: 0,
        reserved: [0, 0, 0, 0, 0, 0],
        data: std::ptr::null()
    };

    loop {
        if let Err(err) = send_cmd(fd, HIDDI_CMD_POLL, &cmd) {
            println!("Poll thread stopping: {err}");
            break;
        }
    }
}

impl Hid {
    pub fn new() -> Result<Hid, i32> {
        // Open a connection to the server.
        let fd = open(c"/dev/io-hid/io-hid", libc::O_RDWR)?;

        // Send the connect message.
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

        send_cmd(fd, HIDDI_CMD_CONNECT, &cmd)?;

        // Start the event polling thread.
        let thandle = thread::spawn(move || { poll_thread(fd); });

        Ok(Hid { fd, thandle: Some(thandle) })
    }
}

impl Drop for Hid {
    fn drop(&mut self) {
        // Close the connection to the server.
        // This should cause the polling thread to exit.
        unsafe { libc::close(self.fd) };

        // Wait for the polling thread to exit.
        if let Some(thandle) = self.thandle.take() {
            thandle.join().unwrap();
        }
    }
}
