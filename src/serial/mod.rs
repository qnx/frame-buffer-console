use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::session::SessionTable;
use crate::qnxmsg::mount::Mount;

/// Information for an open session.
pub struct SerialSession {
    access: u8
}

/// Main server structure.
pub struct SerialServer {
    channel: Channel,
    session_table: SessionTable<SerialSession>,
}

impl SerialServer {
    pub fn new() -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            session_table: SessionTable::<SerialSession>::new()
        };

        // Attach to the mount point.
        let mut mount = Mount::new();
        let _ = mount.attach(&server.channel, "/dev/console", FileType::FILE,
                             0).unwrap();

        server
    }
}
