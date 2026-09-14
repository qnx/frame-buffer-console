/// Module for communicating with io-hid

mod report;
mod device;
mod cmd;
mod input;

use libc;
use std::ffi::CStr;
use std::thread::{self, JoinHandle};
use std::sync::mpsc;
use std::mem::MaybeUninit;

use device::{Device, DeviceType};
use report::Report;
use input::InputReport;

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
#[derive(Copy, Clone, Debug)]
struct HidReport {
    devno: u32,
    report_type: u32,
    flags: u32,
    report_id: u8,
    reserved: [u8; 3]
}

#[repr(C, packed)]
union HidEventPerType {
    insertion: HidDeviceInstance,
    removal: HidDeviceInstance,
    report: HidReport
}

const MAX_REPORT_LEN: usize = 180;

#[repr(C, packed)]
struct HidEvent {
    event_type: u16,
    length: u16,
    pad1: u32,
    event: HidEventPerType,
    reserved: [u8; 2],
    dlen: u16,
    pad2: [u8; 12],
    report: [u8; MAX_REPORT_LEN]
}

/// A connection to the io-hid server.
pub struct Hid {
    fd: i32,
    thandle: Option<JoinHandle<()>>,
    key_receiver: mpsc::Receiver<Option<Key>>,
    pressed_key: Option<Key>
}

#[derive(Clone, PartialEq)]
pub struct Key {
    pub chr: u8,
    pub modifier: u8
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
fn poll_thread(fd: i32, key_sender: mpsc::SyncSender<Option<Key>>) {
    // Construct a poll command.
    // Once a device is attached, the poll command message can return a report, so the reply needs
    // to include a Report structure.
    let cmd = cmd::PollCmd {
        devno: 0,
        param: 0,
        buffer: size_of::<HidEvent>() as u32,
        _pad1: 0,
        _pad2: [0, 0]
    };

    let mut device: Option<Device> = None;

    loop {
        // Get an event.
        let mut event = MaybeUninit::<HidEvent>::uninit();
        if let Err(err) = cmd::send(fd, cmd::POLL, &cmd, Some(&mut event)) {
            println!("Poll thread stopping: {err}");
            break;
        }

        let event = unsafe { event.assume_init() };
        match event.event_type {
            HIDDI_EVT_INSERTION => device_inserted(&event, fd, &mut device),
            HIDDI_EVT_HID_REPORT => report_received(&event, &device, &key_sender),
            HIDDI_EVT_REMOVAL => println!("Device removed"),
            event_type => println!("Received event {event_type}")
        };
    }
}

fn device_inserted(event: &HidEvent, fd: i32, device: &mut Option<Device>) {
    // Do nothing if we already have a keyboard device.
    if device.is_some() {
        return;
    }

    // Extract the insertion event.
    let dev_inst = unsafe { event.event.insertion };
    let report_len = dev_inst.report_desc_len as usize;
    if report_len > MAX_REPORT_LEN {
        println!("Cannot handle device with report length {report_len}");
        return;
    }

    let device_number = dev_inst.devno;

    // Send a report command.
    let cmd = cmd::ReportCmd {
        devno: device_number,
        param: report_len as u32,
        _pad: [0, 0, 0]
    };

    let mut event = MaybeUninit::<HidEvent>::uninit();
    if let Err(err) = cmd::send(fd, cmd::REPORT_DESC, &cmd, Some(&mut event)) {
        println!("Failed to get report: {err}");
        return;
    }

    // Parse the report.
    let event = unsafe { event.assume_init() };
    let report = match Report::new(&event.report, event.dlen as usize, false) {
        Ok(report) => report,
        Err(_) => { println!("Failed to parse report"); return; }
    };

    let (vendid, prodid) = (dev_inst.device_ident.vendor_id,
                            dev_inst.device_ident.product_id);
    println!("Found device device={} vendor={:x} product={:x}", device_number,
             vendid, prodid);

    if report.device_type != DeviceType::Keyboard {
        return;
    }

    // Send an attach command.
    let cmd = cmd::AttachCmd {
        devno: device_number,
        param: 0,
        flags: 0,
        report_type: HID_REPORT_INPUT,
        report_id: report.input_id,
        _pad1: 0,
        _pad2: [ 0, 0 ]
    };

    if let Err(err) = cmd::send(fd, cmd::REPORT_ATTACH, &cmd, NO_REPLY) {
        println!("Failed to attach device: {err}");
        return;
    }

    *device = Some(Device::new(device_number, report.input_report_len));
}

fn report_received(event: &HidEvent, device: &Option<Device>,
                   key_sender: &mpsc::SyncSender::<Option<Key>>) {
    let device = match device {
        None => {
            println!("Report without a device");
            return;
        }
        Some(device) => device
    };

    // Only interested in input reports for the registered device.
    let (devno, report_type) = unsafe {
        (event.event.report.devno, event.event.report.report_type)
    };

    if devno != device.device_number {
        return;
    }

    if report_type != HID_REPORT_INPUT as u32 {
        return;
    }

    let input_report = match InputReport::new(&event, device) {
        Ok(report) => report,
        Err(()) => { return; }
    };

    key_sender.send(input_report.as_key()).unwrap();
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
        let (key_sender, key_receiver) = mpsc::sync_channel(1);
        let thandle = thread::spawn(move || { poll_thread(fd, key_sender); });

        Ok(Hid { fd, thandle: Some(thandle), key_receiver, pressed_key: None })
    }

    /// Retrieve a new key press from io-hid.
    pub fn get_key(&mut self) -> Key {
        loop {
            // Wait for the poll thread to produce a new key.
            let key = match self.key_receiver.recv() {
                Ok(key) => key,
                _ => continue
            };

            // Determine if there was a change.
            let prev_key = std::mem::replace(&mut self.pressed_key, key);
            let new_key = self.pressed_key.clone();

            match prev_key {
                Some(pkey) => {
                    // There is a already a pressed key, check for a change.
                    match new_key {
                        Some(key) => if key != pkey { return key; },
                        None => {}
                    }
                },
                None => {
                    // No key was pressed before.
                    match new_key {
                        Some(key) => { return key; },
                        None => {}
                    }
                }
            }
        }
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
