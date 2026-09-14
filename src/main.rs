mod hid;
mod vga;

use std::thread;
use std::time::Duration;
use hid::Hid;
use vga::Vga;

fn main() {
    let _hid = Hid::new().unwrap();
    let mut vga = Vga::new().unwrap();
    thread::sleep(Duration::from_secs(1));
    let msg = "Hello World!";
    for chr in msg.as_bytes() {
        vga.insert_char_at_cursor(*chr, 0x07, true);
    }
    thread::sleep(Duration::from_secs(1));
}
