/// I/O message handlers for the serial server.

use std::rc::Rc;
use std::cell::RefCell;
use crate::qnxmsg;
use crate::qnxmsg::iomsg::*;
use crate::qnxmsg::transaction::Transaction;
use crate::serial::{SerialServer, SerialSession};

impl IoMsgImpl for SerialServer {
    /// Handle an _IO_CONNECT message with the _IO_CONNECT_OPEN subtype.
    /// Prepares a new session.
    fn connect(&mut self, msg: &ConnectMsg, _path: &str,
               _extra: &ConnectExtra, trans: &mut Transaction)
               -> Result<(), i32> {
        // Add a new session to the table.
        // FIXME:
        // Check permissions.
        let access = (msg.ioflag & 0x7) as u8;
        let session = SerialSession { access };
        let session = Rc::new(RefCell::new(session));
        self.session_table.add(trans.session_id(), &session);
        Ok(())
    }

    /// Handle an _IO_WRITE message.
    /// Bytes are parsed and written to the screen.
    fn write(&mut self, _msg: &WriteMsg, data: &str, trans: &mut Transaction)
             -> Result<(), i32> {
        let session = match self.session_table.get(trans.session_id()) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        let session = session.borrow();
        if (session.access & qnxmsg::ACCESS_WRITE) == 0 {
            return Err(libc::EACCES);
        }

        for chr in data.bytes() {
            self.vga.insert_char_at_cursor(chr, 0x17, true);
        }

        trans.reply(data.len() as i64);
        Ok(())
    }

    /// Handle an _IO_STAT message.
    fn stat(&mut self, _msg: &StatMsg, trans: &mut Transaction)
            -> Result<(), i32> {
        if self.session_table.get(trans.session_id()).is_none() {
            return Err(libc::EBADF);
        }

        let stat = libc::stat {
            st_ino: 1,
            st_size: 0,
            st_dev: 0,
            st_rdev: libc::S_IFCHR,
            st_uid: 0,
            st_gid: 0,
            st_mode: 0o666,
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
        // Get the session object for the duplicated fd.
        let session_ref = match self.session_table.get((msg.pid as i64, msg.coid)) {
            None => { return Err(libc::EBADF); },
            Some(s) => s
        };

        // FIXME:
        // Permission checks for cross-process.

        // Add a reference to the same session to the session table for the
        // current fd.
        let session_ref = session_ref.clone();
        self.session_table.add(trans.session_id(), &session_ref);
        trans.reply(0);
        Ok(())
    }

    /// Handle an _IO_DEVCTL message.
    fn devctl(&mut self, _msg: &DevctlMsg, trans: &mut Transaction)
            -> Result<(), i32> {
        let _session = match self.session_table.get(trans.session_id()) {
            None => { return Err(libc::EBADF); },
            Some(_) => {}
        };

        println!("devctl");
        Err(libc::EINVAL)
    }

    /// Handle an _IO_CLOSE message.
    fn close(&mut self, trans: &Transaction) -> Result<(), i32> {
        if self.session_table.remove(trans.session_id()).is_some() {
            Ok(())
        } else {
            Err(libc::EBADF)
        }
    }
}
