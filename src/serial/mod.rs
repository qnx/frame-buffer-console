mod iomsg;
mod devctl;

use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::session::SessionTable;
use crate::qnxmsg::mount::Mount;
use crate::qnxmsg::message_loop::message_loop;
use crate::vga::Vga;

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
    /// Job control data.
    job: Option<JobControl>
}

/// Information for an open session.
pub struct SerialSession {
    access: u8
}

/// Job control data.
struct JobControl {
    /// Session ID
    /// Not to be confused with open file sessions. This is the POSIX job control's notion of a
    /// session.
    sid: i32,
    /// Process group ID of the foreground process group.
    fpgid: i32,
    /// Process ID of the process that called tcsetsid().
    sess_fd_pid: i64,
    /// Connection ID on which tcsetsid() was called.
    sess_fd_coid: i32
}

impl SerialServer {
    pub fn new(vga: &Vga) -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            session_table: SessionTable::<SerialSession>::new(),
            vga: vga.clone(),
            perms: FilePerms::new(0, 0, 0o600),
            job: None
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

    /// Determine if this device is the controlling terminal for the given session ID.
    fn sid_match(&self, sid: i32) -> bool {
        match &self.job {
            Some(job) => job.sid == sid,
            None => false
        }
    }
}
