mod hid;
mod vga;
mod qnxmsg;
mod serial;

use std::thread;
use std::time::Duration;
use hid::Hid;
use vga::Vga;

fn main() {
    let mut vga = Vga::new().unwrap();
    let _hid = Hid::new(&vga).unwrap();
    thread::sleep(Duration::from_secs(1));
    let msg = "Hello World!";
    for chr in msg.as_bytes() {
        vga.insert_char_at_cursor(*chr, 0x07, true);
    }
    thread::sleep(Duration::from_secs(30));
}
