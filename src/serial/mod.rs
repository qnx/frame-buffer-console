mod iomsg;
mod devctl;
pub mod connection;

use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::session::SessionTable;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::mount::Mount;
use crate::qnxmsg::message_loop::message_loop;
use crate::vga::Vga;
use crate::serial::connection::SerialConnection;

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
    job: Option<JobControl>,
    /// Read transaction waiting for a reply.
    read: Option<Transaction>
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
            perms: FilePerms::new(0, 0, 0o666),
            job: None,
            read: None
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

    pub fn get_connection(&self) -> SerialConnection {
        SerialConnection::new(self.channel)
    }

    /// Determine if this device is the controlling terminal for the given session ID.
    fn sid_match(&self, sid: i32) -> bool {
        match &self.job {
            Some(job) => job.sid == sid,
            None => false
        }
    }

    /// Handle a key event from the HID polling thread.
    /// If a client is waiting on a read request, the key is used in a reply to the client.
    fn key_event(&mut self, key: u8) {
        if let Some(trans) = &mut self.read {
            trans.reply_with_payload(1, &key);
            self.read = None;
        }
    }
}
