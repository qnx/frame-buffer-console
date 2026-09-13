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
    vga.display_char(b'H', 0x07, 10, 10, true);
    vga.display_char(b'e', 0x07, 11, 10, true);
    vga.display_char(b'l', 0x07, 12, 10, true);
    vga.display_char(b'l', 0x07, 13, 10, true);
    vga.display_char(b'o', 0x07, 14, 10, true);
    vga.display_char(b'W', 0x07, 10, 11, true);
    vga.display_char(b'o', 0x07, 11, 11, true);
    vga.display_char(b'r', 0x07, 12, 11, true);
    vga.display_char(b'l', 0x07, 13, 11, true);
    vga.display_char(b'd', 0x07, 14, 11, true);
    vga.display_char(b'!', 0x07, 15, 11, true);
    thread::sleep(Duration::from_secs(1));
}
