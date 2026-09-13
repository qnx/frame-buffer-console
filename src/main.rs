mod hid;
mod vga;

use std::thread;
use std::time::Duration;
use hid::Hid;
use vga::Vga;

fn main() {
    let _hid = Hid::new().unwrap();
    let _vga = Vga::new().unwrap();
    thread::sleep(Duration::from_secs(600));
}
