mod hid;
mod vga;
mod qnxmsg;
mod serial;

use hid::Hid;
use vga::Vga;
use serial::SerialServer;

fn main() {
    let mut vga = Vga::new().unwrap();
    let _hid = Hid::new(&vga).unwrap();
    let mut server = SerialServer::new(&vga);

    let msg = "Welcome to UEFI console!";
    for chr in msg.as_bytes() {
        vga.insert_char_at_cursor(*chr, 0x07, true);
    }

    server.message_loop();
}
