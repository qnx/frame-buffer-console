/// Module for communicating with io-hid

mod report;
mod device;
mod cmd;

use libc;
use std::ffi::CStr;
use std::thread::{self, JoinHandle};
use std::mem::MaybeUninit;
use device::Device;

const HID_VERSION: u16 = 0x100;
const HIDD_VERSION: u16 = 0x100;

const HIDDI_FLAGS_EVTPOLL: u32 = 0x1;
const HIDDI_FLAGS_EXCL_ATTACH: u32 = 0x2;
const HIDDI_FLAGS_ENUMERATE: u32 = 0x4;

const HIDDI_EVT_INSERTION: u16 = 257;
const HIDDI_EVT_REMOVAL: u16 = 258;
const HIDDI_EVT_HID_REPORT: u16 = 260;

const HID_REPORT_INPUT: u16 = 8;
const _HID_REPORT_OUTPUT: u16 = 9;

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
    let cmd = cmd::PollCmd {
        devno: 0,
        param: 0,
        buffer: size_of::<HidReportEvent>() as u32,
        _pad1: 0,
        _pad2: [0, 0]
    };

    let mut device: Option<Device> = None;

    loop {
        // Get an event.
        let mut event = MaybeUninit::<HidReportEvent>::uninit();
        if let Err(err) = cmd::send(fd, cmd::POLL, &cmd, Some(&mut event)) {
            println!("Poll thread stopping: {err}");
            break;
        }

        let event = unsafe { event.assume_init() };
        let (etype, elen) = (event.event.event_type, event.event.length);
        println!("{etype} {elen}");
        match event.event.event_type {
            HIDDI_EVT_INSERTION => device_inserted(&event.event, fd, &mut device),
            HIDDI_EVT_HID_REPORT => report_received(&event, &device),
            HIDDI_EVT_REMOVAL => println!("Device removed"),
            event_type => println!("Received event {event_type}")
        };
    }
}

fn device_inserted(event: &HidEvent, fd: i32, device: &mut Option<Device>) {
    if device.is_some() {
        return;
    }

    // Extract the insertion event.
    let dev_inst = unsafe { event.event.insertion };
    let report_len = dev_inst.report_desc_len as usize;
    if report_len > size_of::<report::Report>() {
        println!("Cannot handle device with report length {report_len}");
        return;
    }

    let mut new_device = Device::new(dev_inst.devno);

    // Send a report command.
    let cmd = cmd::ReportCmd {
        devno: new_device.device_number,
        param: report_len as u32,
        _pad: [0, 0, 0]
    };

    let mut report_event = MaybeUninit::<HidReportEvent>::uninit();
    if let Err(err) = cmd::send(fd, cmd::REPORT_DESC, &cmd, Some(&mut report_event)) {
        println!("Failed to get report: {err}");
        return;
    }

    // Parse the report.
    let report_event = unsafe { report_event.assume_init() };
    if let Err(_) = report::parse(&report_event.report, dev_inst.report_desc_len as usize,
                                  &mut new_device) {
        println!("Failed to parse report");
        return;
    }

    if !new_device.is_keyboard() {
        return;
    }

    println!("Found keyboard {}", new_device.device_number);

    // Send an attach command.
    let cmd = cmd::AttachCmd {
        devno: new_device.device_number,
        param: 0,
        flags: 0,
        report_type: HID_REPORT_INPUT,
        report_id: new_device.input_id,
        _pad1: 0,
        _pad2: [ 0, 0 ]
    };

    if let Err(err) = cmd::send(fd, cmd::REPORT_ATTACH, &cmd, NO_REPLY) {
        println!("Failed to attach device: {err}");
        return;
    }

    *device = Some(new_device);
}

fn report_received(report_event: &HidReportEvent, device: &Option<Device>) {
    let device = match device {
        None => {
            println!("Report without a device");
            return;
        }
        Some(device) => device
    };

    println!("Received report for {:?}", device);
}

impl Hid {
    pub fn new() -> Result<Hid, i32> {
        // Open a connection to the server.
        let fd = open(c"/dev/io-hid/io-hid", libc::O_RDWR)?;

        // Send the connect message.
        let cmd = cmd::ConnectCmd {
            devno: 0,
            param: 0,
            flags: HIDDI_FLAGS_EVTPOLL | HIDDI_FLAGS_EXCL_ATTACH | HIDDI_FLAGS_ENUMERATE,
            vhid: HID_VERSION,
            vhidd: HIDD_VERSION,
            _pad: [0, 0]
        };

        cmd::send(fd, cmd::CONNECT, &cmd, NO_REPLY)?;

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
