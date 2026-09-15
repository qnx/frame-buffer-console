mod iomsg;

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
    channel: Channel,
    session_table: SessionTable<SerialSession>,
    vga: Vga,
    perms: FilePerms
}

impl SerialServer {
    pub fn new(vga: &Vga) -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            session_table: SessionTable::<SerialSession>::new(),
            vga: vga.clone(),
            perms: FilePerms::new(0, 0, 0o600)
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
