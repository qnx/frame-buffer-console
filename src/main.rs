mod hid;
mod vga;

use std::thread;
use std::time::Duration;
use hid::Hid;
use vga::Vga;
use vga::ScreenChar;

fn main() {
    let _hid = Hid::new().unwrap();
    let mut vga = Vga::new().unwrap();
    thread::sleep(Duration::from_secs(1));
    vga.set_char(ScreenChar { chr: b'H', attr: 0x07 }, 10, 10);
    vga.set_char(ScreenChar { chr: b'e', attr: 0x07 }, 11, 10);
    vga.set_char(ScreenChar { chr: b'l', attr: 0x07 }, 12, 10);
    vga.set_char(ScreenChar { chr: b'l', attr: 0x07 }, 13, 10);
    vga.set_char(ScreenChar { chr: b'o', attr: 0x07 }, 14, 10);
    vga.set_char(ScreenChar { chr: b'W', attr: 0x07 }, 10, 11);
    vga.set_char(ScreenChar { chr: b'o', attr: 0x07 }, 11, 11);
    vga.set_char(ScreenChar { chr: b'r', attr: 0x07 }, 12, 11);
    vga.set_char(ScreenChar { chr: b'l', attr: 0x07 }, 13, 11);
    vga.set_char(ScreenChar { chr: b'd', attr: 0x07 }, 14, 11);
    vga.set_char(ScreenChar { chr: b'!', attr: 0x07 }, 15, 11);
    thread::sleep(Duration::from_secs(1));
}
