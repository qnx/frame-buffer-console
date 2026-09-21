mod iomsg;
mod devctl;
mod keybuf;
mod termios;
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
use crate::serial::termios::*;
use crate::hid::key::Key;

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
    notify: Option<NotifyClient>,
    /// Terminal attributes
    attr: TerminalAttributes
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
            notify: None,
            attr: TerminalAttributes::new()
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
    fn key_event(&mut self, key: Key) {
        let mut chr = key.chr;
        if key.is_ctrl() {
            chr = self.handle_ctrl(key);
            if chr == 0 {
                return;
            }
        }

        // Print the character in ECHO mode.
        if self.attr.echo_on() {
            let str = [chr];
            if let Ok(str) = std::str::from_utf8(&str) {
                self.display.insert_at_cursor(&str);
            }
        }

        // Check for a waiting read().
        if let Some(trans) = &mut self.read {
            trans.reply_with_payload(1, &chr);
            self.read = None;
            return;
        }

        // Queue the key.
        self.pending_keys.push(chr);

        // Notify a client that has registered for input events.
        if let Some(notify) = &mut self.notify {
            let mut value = notify.event.sigev_value.sival_ptr as u64;
            value |= libc_missing::_NOTIFY_COND_INPUT as u64;
            notify.event.sigev_value.sival_ptr = value as *mut libc::c_void;

            deliver_event(notify.rcvid, &notify.event).unwrap();
            self.notify = None;
        }
    }

    /// Handle a key combination that includes control.
    /// Returns the character that should be sent to the to a reader, or 0 if the key was
    /// completely consumed by the relevant action.
    fn handle_ctrl(&mut self, key: Key) -> u8 {
        if key.chr == b'c' {
            // Ctrl-C, send SIGINT to the foreground group.
            self.signal_group(libc::SIGINT);
            return 0;
        }

        if key.chr == b'd' {
            // End of transimission.
            if let Some(trans) = &mut self.read {
                trans.reply(0);
                self.read = None;
            }

            // FIXME:
            // What if there isn't a waiting client?
            return 0;
        }

        key.chr
    }

    /// Send a signal to the foreground group, if any.
    fn signal_group(&self, signo: i32) {
        if let Some(job) = &self.job {
            // Note that PROCMGR_SESSION_SIGNAL_PGRP expects the session ID as the first argument,
            // not the ID of the foreground group. Procnto is supposed to know which group is the
            // foreground group, and deliver the signal to it.
            procmgr_session_ext(job.sid, 0, signo,
                                libc_missing::PROCMGR_SESSION_SIGNAL_PGRP).unwrap();
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

/// Safe wrapper around procmgr_session_ext()
fn procmgr_session_ext(sid: libc::pid_t, xid: i32, id: i32, event: u32) -> Result<(), i32> {
    let err = unsafe {
        libc_missing::procmgr_session_ext(0, sid, xid, id, event)
    };

    if err != 0 {
        return Err(unsafe { *libc::__get_errno_ptr() });
    }

    Ok(())
}
