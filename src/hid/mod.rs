/// Module for communicating with io-hid

mod report;
mod device;

use libc;
use std::ptr::*;
use std::ffi::CStr;
use std::thread::{self, JoinHandle};
use std::mem::MaybeUninit;

const _IO_MSG: u16 = 0x113;

const _IOMGR_HID: u16 = 0x13;

const HID_VERSION: u16 = 0x100;
const HIDD_VERSION: u16 = 0x100;

const HIDDI_CMD_CONNECT: u16 = 0x101;
const HIDDI_CMD_POLL: u16 = 0x102;
const HIDDI_CMD_REPORT_ATTACH: u16 = 0x103;
const HIDDI_CMD_REPORT_DESC: u16 = 0x114;

const HIDDI_FLAGS_EVTPOLL: u32 = 0x1;
const HIDDI_FLAGS_EXCL_ATTACH: u32 = 0x2;
const HIDDI_FLAGS_ENUMERATE: u32 = 0x4;

const HIDDI_EVT_INSERTION: u16 = 0x101;
const HIDDI_EVT_REMOVAL: u16 = 0x102;

const HID_REPORT_INPUT: u16 = 0x8;
const HID_REPORT_OUTPUT: u16 = 0x8;

#[repr(C, packed)]
#[derive(Debug)]
struct IoMsg {
    msg_type: u16,
    combine_len: u16,
    mgrid: u16,
    subtype: u16
}

#[repr(C, packed)]
struct ConnectCmd {
    devno: u32,
    param: u32,
    flags: u32,
    vhid: u16,
    vhidd: u16,
    _pad: [u64; 2]
}

#[repr(C, packed)]
struct PollCmd {
    devno: u32,
    param: u32,
    buffer: u32,
    _pad1: u32,
    _pad2: [u64; 2]
}

#[repr(C, packed)]
struct ReportCmd {
    devno: u32,
    param: u32,
    _pad: [u64; 3]
}

#[repr(C, packed)]
struct AttachCmd {
    devno: u32,
    param: u32,
    flags: u32,
    report_type: u16,
    report_id: u8,
    _pad1: u8,
    _pad2: [u64; 2]
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
struct HidDeviceIdent {
    vendor_id: u32,
    product_id: u32,
    version: u32
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
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
    length: u16,
    pad1: u32,
    event: HidEventPerType,
    reserved: [u8; 2],
    dlen: u16,
    pad2: u32,
    data: [u8; 8]
}

#[repr(C, packed)]
struct HidReportEvent {
    event: HidEvent,
    report: report::Report
}

/// A connection to the io-hid server.
pub struct Hid {
    fd: i32,
    thandle: Option<JoinHandle<()>>
}

const NO_REPLY: Option<&mut i32> = None;

/// Send a command to the io-hid server.
/// The command comprises a _IO_MSG header and a command structure.
fn send_cmd<T, U>(fd: i32, cmd_id: u16, cmd: &T, reply: Option<&mut U>) -> Result<i64, i32> {
    let hdr = IoMsg {
        msg_type: _IO_MSG,
        subtype: cmd_id,
        mgrid: _IOMGR_HID,
        combine_len: size_of::<IoMsg>() as u16 + size_of::<PollCmd>() as u16
    };

    let result = unsafe {
        // Construct two-part message.
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
            Some(u) => (addr_of!(*u) as *mut libc::c_void,
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
    // Once a device is attached, the poll command message can return a report, so the reply needs
    // to include a Report structure.
    let cmd = PollCmd {
        devno: 0,
        param: 0,
        buffer: size_of::<HidReportEvent>() as u32,
        _pad1: 0,
        _pad2: [0, 0]
    };

    loop {
        // Get an event.
        let mut event = MaybeUninit::<HidReportEvent>::uninit();
        if let Err(err) = send_cmd(fd, HIDDI_CMD_POLL, &cmd, Some(&mut event)) {
            println!("Poll thread stopping: {err}");
            break;
        }

        let event = unsafe { event.assume_init() };
        let (etype, elen) = (event.event.event_type, event.event.length);
        println!("{etype} {elen}");
        match event.event.event_type {
            HIDDI_EVT_INSERTION => device_inserted(&event.event, fd),
            HIDDI_EVT_REMOVAL => println!("Device removed"),
            event_type => println!("Received event {event_type}")
        };
    }
}

fn device_inserted(event: &HidEvent, fd: i32) {
    // Extract the insertion event.
    let dev_inst = unsafe { event.event.insertion };
    let report_len = dev_inst.report_desc_len as usize;
    if report_len > size_of::<report::Report>() {
        println!("Cannot handle device with report length {report_len}");
        return;
    }

    let mut device = device::Device::new(dev_inst.devno);

    // Send a report command.
    let cmd = ReportCmd {
        devno: device.device_number,
        param: report_len as u32,
        _pad: [0, 0, 0]
    };

    let mut report_event = MaybeUninit::<HidReportEvent>::uninit();
    if let Err(err) = send_cmd(fd, HIDDI_CMD_REPORT_DESC, &cmd, Some(&mut report_event)) {
        println!("Failed to get report: {err}");
        return;
    }

    // Parse the report.
    let report_event = unsafe { report_event.assume_init() };
    if let Err(_) = report::parse(&report_event.report, dev_inst.report_desc_len as usize,
                                  &mut device) {
        println!("Failed to parse report");
        return;
    }

    if !device.is_keyboard() {
        return;
    }

    println!("Found keyboard {}", device.device_number);

    // Send an attach command.
    let cmd = AttachCmd {
        devno: device.device_number,
        param: 0,
        flags: 0,
        report_type: HID_REPORT_INPUT,
        report_id: device.input_id,
        _pad1: 0,
        _pad2: [ 0, 0 ]
    };

    if let Err(err) = send_cmd(fd, HIDDI_CMD_REPORT_ATTACH, &cmd, NO_REPLY) {
        println!("Failed to attach device: {err}");
        return;
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
            _pad: [0, 0]
        };

        send_cmd(fd, HIDDI_CMD_CONNECT, &cmd, NO_REPLY)?;

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
