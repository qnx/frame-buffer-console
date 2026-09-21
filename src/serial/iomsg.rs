/// I/O message handlers for the serial server.

use std::rc::Rc;
use std::cell::RefCell;
use crate::qnxmsg;
use crate::qnxmsg::iomsg::*;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::Pulse;
use crate::serial::{SerialServer, SerialConnection, NotifyClient};
use crate::libc_missing;
use crate::hid::key::Key;

impl IoMsgImpl for SerialServer {
    /// Handle an _IO_CONNECT message with the _IO_CONNECT_OPEN subtype.
    /// Prepares a new connection.
    fn connect(&mut self, msg: &ConnectMsg, _path: &str,
               _extra: &ConnectExtra, trans: &mut Transaction)
               -> Result<(), i32> {
        // Check that the client is allowed to connect.
        let access = (msg.ioflag & 0x7) as u8;
        if !trans.check_credentials(&self.perms, access) {
            return Err(libc::EPERM);
        }

        // Add a new connection to the table.
        let connection = SerialConnection { access };
        let connection = Rc::new(RefCell::new(connection));
        self.connection_table.add(trans.connection_id(), &connection);
        Ok(())
    }

    /// Handle an _IO_WRITE message.
    /// Bytes are parsed and written to the screen.
    fn write(&mut self, _msg: &WriteMsg, data: &str, trans: &mut Transaction)
             -> Result<(), i32> {
        let connection = match self.connection_table.get(trans.connection_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        let connection = connection.borrow();
        if (connection.access & qnxmsg::ACCESS_WRITE) == 0 {
            return Err(libc::EACCES);
        }

        self.display.insert_at_cursor(data);
        trans.reply(data.len() as i64);
        Ok(())
    }

    /// Handle an _IO_READ message.
    fn read(&mut self, msg: &ReadMsg, trans: &mut Transaction)
            -> Result<(), i32> {
        let connection = match self.connection_table.get(trans.connection_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        // Check for read permission.
        let connection = connection.borrow();
        if (connection.access & qnxmsg::ACCESS_READ) == 0 {
            return Err(libc::EACCES);
        }

        // Ensure that the reader is from the foreground process group.
        let info = trans.get_client_info()?;
        if !self.sid_match(info.sid) {
            return Err(libc::ENOTTY);
        }

        // FIXME:
        // Send SIGTTIN?
        if (info.flags & libc::_NTO_CI_BKGND_PGRP) != 0 {
            return Err(libc::ENOTTY);
        }

        if self.read.is_some() {
            // FIXME:
            // Can there be more than one client request reading at the same time?
            return Err(libc::EBUSY);
        }

        // If there are no pending keys, then remember the transaction so that it can be replied to
        // once a key event occurs.
        if self.pending_keys.is_empty() {
            self.read = Some(trans.reply_later());
            return Ok(());
        }

        // Reply with pending keys.
        let (data, len) = self.pending_keys.get_data(msg.nbytes as usize);
        trans.reply_with_payload_and_size(len as i64, data, len);
        self.pending_keys.pop(len);
        Ok(())
    }

    /// Handle an _IO_STAT message.
    fn stat(&mut self, _msg: &StatMsg, trans: &mut Transaction)
            -> Result<(), i32> {
        if self.connection_table.get(trans.connection_id()).is_none() {
            return Err(libc::EBADF);
        }

        let stat = libc::stat {
            st_ino: 1,
            st_size: 0,
            st_dev: 0,
            st_rdev: libc::S_IFCHR,
            st_uid: self.perms.uid,
            st_gid: self.perms.gid,
            st_mode: self.perms.get_mode() | libc::S_IFCHR,
            st_nlink: 0,
            st_blocksize: 0,
            st_nblocks: 0,
            st_blksize: 0,
            st_blocks: 0,
            st_mtim: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            st_atim: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            st_ctim: libc::timespec { tv_sec: 0, tv_nsec: 0 },
            __old_st_mtime: 0,
            __old_st_atime: 0,
            __old_st_ctime: 0
        };

        trans.reply_with_payload(0, &stat);
        Ok(())
    }

    /// Handle an _IO_DUP message.
    fn dup(&mut self, msg: &DupMsg, trans: &mut Transaction)
           -> Result<(), i32> {
        // Get the connection object for the duplicated fd.
        let connection_ref = match self.connection_table.get((msg.pid as i64, msg.coid)) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        // FIXME:
        // Permission checks for cross-process.

        // Add a reference to the same connection to the connection table for the
        // current fd.
        let connection_ref = connection_ref.clone();
        self.connection_table.add(trans.connection_id(), &connection_ref);
        trans.reply(0);
        Ok(())
    }

    /// Handle an _IO_DEVCTL message.
    fn devctl(&mut self, msg: &DevctlMsg, payload: &[u8], trans: &mut Transaction)
            -> Result<(), i32> {
        let connection = match self.connection_table.get(trans.connection_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        let access = connection.borrow().access;
        self.devctl(msg.dcmd, payload, access, trans)
    }

    /// Handle an _IO_OPENFD message.
    /// An openfd() call is a request to make the file descriptor on which the message was sent a
    /// duplicate of another file descriptor, potentially from another process, identified by the
    /// message's info field. A dup handle is used to verify that the requesting process was
    /// granted access to the original descriptor.
    fn openfd(&mut self, msg: &OpenfdMsg, trans: &mut Transaction)
              -> Result<(), i32> {
        // Current fd should not be associated with a connection (i.e., should not have been opened
        // previously).
        if self.connection_table.get(trans.connection_id()).is_some() {
            return Err(libc::EBUSY);
        };

        // Check permissions.
        let dstpid = trans.info.pid64();
        let handle = msg.handle;
        if handle != 0 {
            // Check for a valid dup handle.
            let handle = msg.handle;
            let err = unsafe {
                libc_missing::ConnectDupFd_r(libc_missing::_NTO_DUPFD_VERIFY,
                                             &dstpid as *const i64,
                                             &handle as *const u64,
                                             std::ptr::null_mut(),
                                             &msg.info as *const libc_missing::_msg_info_hdr)
            };

            if err != 0 {
                return Err(err);
            }
        } else if dstpid == 1 {
            // Procnto sends a message without a dup handle when handling tcsetsid().
        } else {
            return Err(libc::EPERM);
        }

        // Check that the client is allowed to connect.
        let access = (msg.ioflag & 0x7) as u8;
        if !trans.check_credentials(&self.perms, access) {
            return Err(libc::EPERM);
        }

        // Add a new connection to the table.
        let connection = SerialConnection { access };
        let connection = Rc::new(RefCell::new(connection));
        self.connection_table.add(trans.connection_id(), &connection);
        trans.reply(0);
        Ok(())
    }

    /// Handle an _IO_NOTIFY message.
    /// Queries the server for any pending keys, allowing for a notification to be armed if no keys
    /// are available.
    fn notify(&mut self, msg: &NotifyMsg, _fds: &[libc::pollfd], trans: &mut Transaction)
              -> Result<(), i32> {
        // For now, we only accept a single fd with an input POLLARM action.
        let nfds = msg.nfds;
        let action = msg.action;
        let flags = msg.flags &
            (libc_missing::_NOTIFY_COND_INPUT |
             libc_missing::_NOTIFY_COND_OUTPUT |
             libc_missing::_NOTIFY_COND_OBAND);

        if nfds != 1 {
            return Err(libc::EINVAL);
        }

        let arm = match action {
            libc_missing::_NOTIFY_ACTION_POLLARM => true,
            libc_missing::_NOTIFY_ACTION_POLL => false,
            _ => { return Err(libc::EINVAL); }
        };

        if flags != libc_missing::_NOTIFY_COND_INPUT {
            return Err(libc::EINVAL);
        }

        // Only one notify event.
        if self.notify.is_some() {
            return Err(libc::EBUSY);
        }

        // Reply to the client.
        // If there are pending keys, then set the satisfied input condition.
        // Otherwise, store the notification event so that it can be emitted later.
        let mut reply = NotifyReply {
            zero: 0,
            flags: 0
        };

        if !self.pending_keys.is_empty() {
            reply.flags = libc_missing::_NOTIFY_COND_INPUT
        } else if arm {
            self.notify = Some(NotifyClient { rcvid: trans.get_rcvid(), event: msg.event });
        } else {
            // Nothing to do.
        }

        trans.reply_with_payload(0, &reply);
        Ok(())
    }

    /// Handle an _IO_CLOSE message.
    fn close(&mut self, trans: &Transaction) -> Result<(), i32> {
        match self.connection_table.remove(trans.connection_id()) {
            Some(_) => {
                if let Some(job) = &mut self.job {
                    // Determine if the close comes from the fd that registered the connection.
                    let (pid, coid) = trans.connection_id();
                    if (job.sess_fd_pid == pid) && (job.sess_fd_coid == coid) {
                        // Connection closed.
                        println!("Session closed");
                        self.job = None;
                    }
                }
                Ok(())
            },
            None => Err(libc::EBADF)
        }
    }

    /// Handle pulses.
    fn pulse(&mut self, pulse: &Pulse) {
        let (code, scoid) = (pulse.code, pulse.scoid);
        if scoid != 0 {
            // Ignore external pulses.
            return;
        }

        match code {
            0 => self.key_event(Key::from_pulse(pulse)),
            _ => { println!("Unhandled pulse {code}"); }
        };
    }
}
