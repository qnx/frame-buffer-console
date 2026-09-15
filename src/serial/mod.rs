mod iomsg;
mod devctl;

use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::session::SessionTable;
use crate::qnxmsg::mount::Mount;
use crate::qnxmsg::message_loop::message_loop;
use crate::vga::Vga;

/// Information for an open session.
pub struct SerialSession {
    access: u8
}

/// Main server structure.
pub struct SerialServer {
    /// Channel on which to receive messages.
    channel: Channel,
    /// Table of connected client sessions.
    session_table: SessionTable<SerialSession>,
    /// Display access structure.
    vga: Vga,
    /// POSIX IDs and file permissions for the device node.
    perms: FilePerms,
    /// Job control session ID.
    sid: Option<i32>
}

impl SerialServer {
    pub fn new(vga: &Vga) -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            session_table: SessionTable::<SerialSession>::new(),
            vga: vga.clone(),
            perms: FilePerms::new(0, 0, 0o600),
            sid: None
        };

        // Attach to the mount point.
        let mut mount = Mount::new();
        let _ = mount.attach(&server.channel, "/dev/uefi", FileType::FILE,
                             0).unwrap();

        server
    }

    /// Receive and handle messages on the server's channel.
    pub fn message_loop(&mut self) {
        message_loop(self, self.channel);
    }
}
