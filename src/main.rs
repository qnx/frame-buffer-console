mod hid;
mod vga;
mod qnxmsg;
mod serial;
mod libc_missing;

use hid::Hid;
use vga::Vga;
use serial::SerialServer;

fn main() {
    let mut vga = Vga::new().unwrap();
    let mut server = SerialServer::new(&vga);
    let _hid = Hid::new(&server).unwrap();

    let msg = "Welcome to UEFI console!\n";
    vga.insert_at_cursor(msg);

    server.message_loop();
}
