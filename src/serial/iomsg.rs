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

use std::rc::Rc;
use std::cell::RefCell;
use crate::qnxmsg;
use crate::qnxmsg::iomsg::*;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::Pulse;
use crate::serial::{SerialServer, SerialConnection, NotifyClient};
use crate::libc_missing;
use crate::hid::key::Key;

/// I/O message handlers for the serial server.
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

        let block_io = (msg.ioflag & (libc::O_NONBLOCK as u32)) == 0;

        // Add a new connection to the table.
        let connection = SerialConnection { access, block_io };
        let connection = Rc::new(RefCell::new(connection));
        self.connection_table.add(trans.connection_id(), &connection);
        Ok(())
    }

    /// Handle an _IO_WRITE message.
    /// Bytes are parsed and written to the screen.
    fn write(&mut self, msg: &WriteMsg, data: &[u8], trans: &mut Transaction)
             -> Result<(), i32> {
        let connection = match self.connection_table.get(trans.connection_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        // Verify write access.
        let connection = connection.borrow();
        if (connection.access & qnxmsg::ACCESS_WRITE) == 0 {
            return Err(libc::EACCES);
        }

        // Handle the bytes available in the internal buffer.
        let mut write_len = 0;
        self.display.insert_at_cursor(data);
        write_len += data.len();

        // Handle additional bytes in the message.
        let mut offset = size_of::<WriteMsg>() + data.len();
        while write_len < msg.nbytes as usize {
            let mut data: [u8; 1024] = [0; 1024];
            let nbytes = trans.read(&mut data, offset)?;
            self.display.insert_at_cursor(&data[0..nbytes]);
            write_len += nbytes;
            offset += nbytes;
        }

        trans.reply(write_len as i64);
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
        if self.input_buffer.is_empty() {
            if !connection.block_io {
                return Err(libc::EAGAIN);
            }

            self.read = Some(trans.reply_later());
            return Ok(());
        }

        // Reply with pending keys.
        let (data, len) = self.input_buffer.get_data(msg.nbytes as usize);
        trans.reply_with_payload_and_size(len as i64, data, len);
        self.input_buffer.pop(len);
        Ok(())
    }

    /// Handle an _IO_STAT message.
    fn stat(&mut self, _msg: &StatMsg, trans: &mut Transaction)
            -> Result<(), i32> {
        if self.connection_table.get(trans.connection_id()).is_none() {
            return Err(libc::EBADF);
        }

        // FIXME:
        // There is currently no safe way to instantiate a stat structure.
        let mut stat = unsafe {
            std::mem::MaybeUninit::<libc::stat>::zeroed().assume_init()
        };

        stat.st_dev = self.devno;
        stat.st_ino = 1;
        stat.st_rdev = libc::S_IFCHR;
        stat.st_uid = self.perms.uid;
        stat.st_gid = self.perms.gid;
        stat.st_mode = self.perms.get_mode() | libc::S_IFCHR;

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
        let connection_ref = match self.connection_table.get(trans.connection_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        let connection_ref = connection_ref.clone();
        self.devctl(msg.dcmd, payload, trans, &connection_ref)
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

        let block_io = (msg.ioflag & (libc::O_NONBLOCK as u32)) == 0;

        // Add a new connection to the table.
        let connection = SerialConnection { access, block_io };
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

        if !self.input_buffer.is_empty() {
            reply.flags = libc_missing::_NOTIFY_COND_INPUT
        } else if arm {
            let (pid, coid) = trans.connection_id();
            self.notify = Some(NotifyClient {
                rcvid: trans.get_rcvid(),
                pid,
                coid,
                event: msg.event
            });
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
                let (pid, coid) = trans.connection_id();
                if let Some(read_trans) = &mut self.read {
                    // Release a pending read on this connection.
                    let read_id = read_trans.connection_id();
                    if read_id.0 == pid && read_id.1 == coid {
                        read_trans.error(libc::EBADF);
                        self.read = None
                    }
                }

                if let Some(notify) = &mut self.notify {
                    // Release a pending read on this connection.
                    if notify.pid == pid && notify.coid == coid {
                        self.notify = None
                    }
                }

                if let Some(job) = &mut self.job {
                    // Determine if the close comes from the fd that registered the connection.
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
