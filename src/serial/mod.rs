mod iomsg;
mod devctl;
mod keybuf;
pub mod connection;

use crate::libc_missing;
use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::session::SessionTable;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::mount::Mount;
use crate::qnxmsg::message_loop::message_loop;
use crate::display::Display;
use crate::serial::keybuf::KeyBuf;
use crate::serial::connection::SerialConnection;

/// Main server structure.
pub struct SerialServer {
    /// Channel on which to receive messages.
    channel: Channel,
    /// Table of connected client sessions.
    session_table: SessionTable<SerialSession>,
    /// Display access structure.
    display: Display,
    /// POSIX IDs and file permissions for the device node.
    perms: FilePerms,
    /// Job control data.
    job: Option<JobControl>,
    /// Key events that have not been read.
    pending_keys: KeyBuf,
    /// Read transaction waiting for a reply.
    read: Option<Transaction>,
    /// A client event to emit when input is available.
    notify: Option<NotifyClient>
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

/// A client event registered for notification when input is available
struct NotifyClient {
    rcvid: i64,
    event: libc::sigevent
}

impl SerialServer {
    pub fn new(display: &Display) -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            session_table: SessionTable::<SerialSession>::new(),
            display: display.clone(),
            perms: FilePerms::new(0, 0, 0o666),
            job: None,
            pending_keys: KeyBuf::new(),
            read: None,
            notify: None
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
    /// Otherwise, the key is queued for a future read().
    fn key_event(&mut self, key: u8) {
        // Check for a waiting read().
        if let Some(trans) = &mut self.read {
            trans.reply_with_payload(1, &key);
            self.read = None;
            return;
        }

        // Queue the key.
        self.pending_keys.push(key);

        // Notify a client that has registered for input events.
        if let Some(notify) = &mut self.notify {
            let mut value = notify.event.sigev_value.sival_ptr as u64;
            value |= libc_missing::_NOTIFY_COND_INPUT as u64;
            notify.event.sigev_value.sival_ptr = value as *mut libc::c_void;

            deliver_event(notify.rcvid, &notify.event).unwrap();
            self.notify = None;
        }
    }
}

/// Safe wrapper around MsgDeliverEvent_r().
fn deliver_event(rcvid: i64, event: &libc::sigevent) -> Result<(), i32> {
    let err = unsafe {
        libc::MsgDeliverEvent_r(rcvid, event as *const libc::sigevent)
    };

    if err != 0 {
        return Err(err);
    }

    Ok(())
}
