mod hid;
mod display;
mod qnxmsg;
mod serial;
mod libc_missing;

use hid::Hid;
use display::Display;
use serial::SerialServer;

fn main() {
    let mut display = Display::new().unwrap();
    let mut server = SerialServer::new(&display);
    let _hid = Hid::new(&server).unwrap();

    let msg = b"Welcome to UEFI console!\n";
    display.insert_at_cursor(msg);

    server.message_loop();
}
