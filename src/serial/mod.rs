mod iomsg;
mod devctl;
mod inputbuffer;
mod termios;
mod lineeditor;

use crate::libc_missing;
use crate::qnxmsg::*;
use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::connection::ConnectionTable;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::mount::Mount;
use crate::qnxmsg::message_loop::message_loop;
use crate::display::Display;
use crate::serial::inputbuffer::InputBuffer;
use crate::serial::lineeditor::LineEditor;
use crate::serial::termios::*;
use crate::hid::key::*;

/// Main server structure.
pub struct SerialServer {
    /// Channel on which to receive messages.
    channel: Channel,
    /// Table of connected client connections.
    connection_table: ConnectionTable<SerialConnection>,
    /// Display access structure.
    display: Display,
    /// POSIX IDs and file permissions for the device node.
    perms: FilePerms,
    /// Job control data.
    job: Option<JobControl>,
    /// Characters that have not been read.
    input_buffer: InputBuffer,
    /// LineEditor line editor.
    lineedit: LineEditor,
    /// Read transaction waiting for a reply.
    read: Option<Transaction>,
    /// A client event to emit when input is available.
    notify: Option<NotifyClient>,
    /// Terminal attributes
    attr: TerminalAttributes
}

/// Information for an open connection.
pub struct SerialConnection {
    access: u8,
    block_io: bool
}

/// Job control data.
struct JobControl {
    /// Session ID.
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
    pid: i64,
    coid: i32,
    event: libc::sigevent
}

impl SerialServer {
    pub fn new(display: &Display) -> Self {
        // Create the server object.
        let server = SerialServer {
            channel: Channel::new(0).unwrap(),
            connection_table: ConnectionTable::<SerialConnection>::new(),
            display: display.clone(),
            perms: FilePerms::new(0, 0, 0o666),
            job: None,
            input_buffer: InputBuffer::new(),
            lineedit: LineEditor::new(),
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

    pub fn get_channel(&self) -> Channel {
        self.channel
    }

    pub fn emit_key_event(channel: &Channel, key: Key) -> Result<(), i32> {
        let chid = channel.chid;
        let err = unsafe {
            libc_missing::MsgSendPulse_r(-chid, -1, 0, key.to_pulse_value())
        };

        if err != 0 {
            return Err(err);
        }

        Ok(())
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
    fn key_event(&mut self, mut key: Key) {
        if self.attr.is_canonical() {
            // Canonical mode.
            // Let the line editor handle the key first.
            key = match self.lineedit.key_event(key, &mut self.display) {
                Some(key) => key,
                None => { return; }
            };
        }

        let chr = key.chr;
        if key.is_ctrl() {
            self.handle_ctrl(key);
            return;
        }

        if key.is_special() {
            self.handle_special(key);
            return;
        }

        // Print the character in ECHO mode.
        if self.attr.echo_on() {
            let chrs = [chr];
            self.display.insert_at_cursor(&chrs);
        }

        let chrs: [u8; 1] = [chr];
        self.data_available(Some(&chrs));
    }

    /// Handle a key combination that includes control.
    fn handle_ctrl(&mut self, key: Key) {
        match key.chr {
            b'a' => {
                // Ctrl-A, send Start Of Heading (1).
                let chrs: [u8; 1] = [1];
                self.data_available(Some(&chrs));
            }
            b'c' => {
                // Ctrl-C, send SIGINT to the foreground group.
                self.signal_foreground_group(libc::SIGINT);
            }
            b'd' => {
                // Ctrl-D, end of transimission.
                let chrs: [u8; 0] = [];
                self.data_available(Some(&chrs));
            }
            b'e' => {
                // Ctrl-E, send Enquiry (5).
                let chrs: [u8; 1] = [5];
                self.data_available(Some(&chrs));
            }
            _ => { println!("Unhandled control {}", key.chr); }
        }
    }

    /// Handle a key marked with the special modifier.
    fn handle_special(&mut self, key: Key) {
        match key.chr {
            KEY_LEFT_ARROW => self.data_available(Some(&[0x1b, b'[', b'D'])),
            KEY_RIGHT_ARROW => self.data_available(Some(&[0x1b, b'[', b'C'])),
            KEY_UP_ARROW => self.data_available(Some(&[0x1b, b'[', b'A'])),
            KEY_DOWN_ARROW => self.data_available(Some(&[0x1b, b'[', b'B'])),
            KEY_CANON_EOL => self.canon_eol(),
            KEY_CANON_EOT => self.canon_eot(),
            _ => {}
        }
    }

    /// Handle end-of-line (typically ENTER) in canonical mode.
    /// EOL makes the current line buffer available as input and moves the editor to the next line.
    fn canon_eol(&mut self) {
        // Make the line buffer available to readers.
        assert!(self.attr.is_canonical());
        self.data_available(None);

        // Start editing the next line.
        // Note that scrolling can affect the resulting line number.
        self.lineedit.set_line(self.display.get_cursor_pos().1);
    }

    /// Handle end-of-transmission (typically Ctrl-D) in canonical mode.
    /// Makes the contents of the line editor's buffer available to readers.
    fn canon_eot(&mut self) {
        // Make the line buffer available to readers.
        assert!(self.attr.is_canonical());
        self.data_available(None);
    }

    /// Called when new data is ready to be read from the device.
    /// If there is a pending read transaction, the transaction is replied to. Any remaining data
    /// is stored in the input buffer for future reads.
    /// If chrs is None then use the line editor's buffer as the source of data.
    fn data_available(&mut self, chrs: Option<&[u8]>) {
        // The use of Option<&[u8]> avoids the restriction of the borrow checkers, which prevents
        // a mutable reference to self and a non-mutable reference to the line editor's buffer from
        // being passed to this function.
        let chrs = match chrs {
            Some(chrs) => chrs,
            None => self.lineedit.get_line()
        };

        // Check for a waiting read().
        let mut len = chrs.len();
        let mut pos = 0;
        if let Some(trans) = &mut self.read {
            // Reply to the client with as many bytes as it requested.
            assert!(self.input_buffer.is_empty());
            let reply_len = std::cmp::min(len, trans.get_reply_size());
            trans.reply_with_payload_and_size(reply_len as i64, chrs, reply_len);
            self.read = None;

            len -= reply_len;
            if len == 0 {
                return;
            }

            pos += reply_len;
        }

        // Queue the remaining data.
        self.input_buffer.push(&chrs[pos..]);

        // Notify a client that has registered for input events.
        if let Some(notify) = &mut self.notify {
            let mut value = notify.event.sigev_value.sival_ptr as u64;
            value |= libc_missing::_NOTIFY_COND_INPUT as u64;
            notify.event.sigev_value.sival_ptr = value as *mut libc::c_void;

            deliver_event(notify.rcvid, &notify.event).unwrap();
            self.notify = None;
        }
    }

    /// Send a signal to the foreground group, if any.
    fn signal_foreground_group(&self, signo: i32) {
        if let Some(job) = &self.job {
            // Note that PROCMGR_SESSION_SIGNAL_PGRP expects the session ID as the first argument,
            // not the ID of the foreground group. Procnto is supposed to know which group is the
            // foreground group, and deliver the signal to it.
            procmgr_session_ext(job.sid, 0, signo,
                                libc_missing::PROCMGR_SESSION_SIGNAL_PGRP).unwrap();
        }
    }

    /// Send a signal to the background group.
    /// The function returns an error if and only if the signal was not blocked by the receiver.
    /// This is used for handling POSIX semantics for SIGTTOU/SIGTTIN, in which a process that
    /// blocks the signal is allowed to perform the requested operation.
    fn signal_background_group(&self, pid: i32, tid: i32, signo: i32) ->
        Result<(), i32> {
        // Note that despite its name, PROCMGR_SESSION_SIGNAL_PID will signal the process
        // group, as required.
        match procmgr_session_ext(pid, tid, signo,
                                  libc_missing::PROCMGR_SESSION_SIGNAL_PID) {
            Ok(()) => {
                // Signal was sent, do not perform the requested operation.
                Err(libc::EINTR)
            }
            Err(libc::EIO) => {
                // Signal was blocked, proceed to change group.
                Ok(())
            }
            Err(err) => {
                // Some other error.
                Err(err)
            }
        }
    }
}

/// Safe wrapper around MsgDeliverEvent_r().
fn deliver_event(rcvid: i64, event: &libc::sigevent) -> Result<(), i32> {
    let err = unsafe {
        libc_missing::MsgDeliverEvent_r(rcvid, event as *const libc::sigevent)
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
