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

    let release = confstr(libc_missing::_CS_RELEASE);
    let host = confstr(libc_missing::_CS_HOSTNAME);
    let machine = confstr(libc_missing::_CS_MACHINE);
    let msg = format!("\x1b[1;31mWelcome to QNX {release} on {host} {machine}\n");
    display.insert_at_cursor(msg.as_bytes());

    server.message_loop();
}

fn confstr(name: i32) -> String {
    let mut buf: [u8; 64] = [0; 64];
    let size = unsafe {
        let addr = std::ptr::addr_of_mut!(buf);
        libc::confstr(name, addr as *mut libc::c_char, 64)
    };

    if size == 0 {
        return String::new()
    }

    let size = std::cmp::min(size - 1, 64);
    String::from_utf8_lossy(&buf[0..size]).to_string()
}
