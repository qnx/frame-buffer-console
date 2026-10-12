// Copyright (c) 2026 BlackBerry Limited.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod hid;
mod display;
mod qnxmsg;
mod serial;
mod libc_missing;

use std::env;
use hid::Hid;
use display::Display;
use serial::SerialServer;

fn main() {
    // Parse command-line arguments.
    let mut double_buffer = false;

    let args = env::args_os().skip(1);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str().unwrap_or_default() {
            "--double-buffer" => double_buffer = true,
            _ => {}
        };
    }

    // Create the top-level objects.
    let mut display = Display::new(double_buffer).unwrap();
    let mut server = SerialServer::new(&display);
    let _hid = Hid::new(&server).unwrap();

    // Display a welcome message.
    let release = confstr(libc_missing::_CS_RELEASE);
    let host = confstr(libc_missing::_CS_HOSTNAME);
    let machine = confstr(libc_missing::_CS_MACHINE);
    let msg = format!("\x1b[1;31mWelcome to QNX {release} on {host} {machine}\n\x1b[0m");
    display.insert_at_cursor(msg.as_bytes());

    // Run the server's message loop.
    server.message_loop();
}

/// Safe wrapper for the C library's confstr() function.
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
