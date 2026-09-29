use crate::serial::*;
use crate::qnxmsg::*;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::iomsg::DevctlReply;
use crate::libc_missing;

const DCMD_ALL_GETFLAGS: u32 = 0x40040101;
const DCMD_ALL_SETFLAGS: u32 = 0x80040102;
const DCMD_CHR_TCSETSID: u32 = 0x80040308;
const DCMD_CHR_TTYINFO: u32 = 0x4024030a;
const DCMD_CHR_ISATTY: u32 = 0x318;
const DCMD_CHR_TCSETPGRP: u32 = 0x80047476;
const DCMD_CHR_TCGETPGRP: u32 = 0x40047477;
const DCMD_CHR_TCGETATTR : u32 = 0x404c7413;
const DCMD_CHR_TCSETATTR : u32 = 0x804c7414;
const DCMD_CHR_TCSETATTRD : u32 = 0x804c7415;
const DCMD_CHR_TCGETSIZE : u32 = 0x40087468;
const DCMD_CHR_TCSETSIZE : u32 = 0x80087467;

#[repr(C, packed)]
struct GetflagsReply {
    hdr: DevctlReply,
    oflag: i32
}

#[repr(C, packed)]
struct TcgetprpReply {
    hdr: DevctlReply,
    pgid: i32
}

#[repr(C, packed)]
struct TcgetattrReply {
    hdr: DevctlReply,
    attr: libc::termios
}

#[repr(C, packed)]
struct TcgetsizeReply {
    hdr: DevctlReply,
    size: libc::winsize
}

#[repr(C, packed)]
struct TtyinfoReply {
    opencount: u32,
    pad1: u32,
    ttyname: [char; 9],
    pad2: [u8; 23]
}

impl DevctlReply {
    fn new<T>(ret_val: i32) -> Self {
        DevctlReply {
            zero: 0,
            ret_val,
            nbytes: size_of::<T>() as u32,
            zero2: 0
        }
    }
}

/// Safely convert an array of bytes in a message's payload to a structured
/// message.
fn transmute_payload<T>(payload: &[u8]) -> Result<&T, ()> {
    if payload.len() < std::mem::size_of::<T>() {
        // Payload too short for structure.
        return Err(());
    }

    let (head, t, _) = unsafe { payload.align_to::<T>() };
    if !head.is_empty() {
        // Payload not aligned.
        return Err(());
    }

    Ok(&t[0])
}

impl SerialServer {
    /// Top-level devctl dispatcher.
    pub fn devctl(&mut self, dcmd: u32, payload: &[u8], access: u8,
                  trans: &mut Transaction) -> Result<(), i32> {
        // For historical reasons, all character device devctl()s can be performed on read-only
        // file descriptors.
        if (access & ACCESS_READ) == 0 {
            let pid = trans.info.pid;
            println!("devctl {:x} from {}: no permissions {:o}", dcmd, pid, access);
            return Err(libc::EPERM);
        }

        match dcmd {
            DCMD_ALL_GETFLAGS => self.getflags(access, trans),
            DCMD_ALL_SETFLAGS => self.setflags(payload, trans),
            DCMD_CHR_ISATTY => { trans.reply(0); Ok(()) },
            DCMD_CHR_TCSETSID => self.tcsetsid(payload, trans),
            DCMD_CHR_TCSETPGRP => self.tcsetpgrp(payload, trans),
            DCMD_CHR_TCGETPGRP => self.tcgetpgrp(trans),
            DCMD_CHR_TCGETATTR => self.tcgetattr(trans),
            DCMD_CHR_TCSETATTR => self.tcsetattr(payload, trans),
            DCMD_CHR_TCSETATTRD => self.tcsetattr(payload, trans),
            DCMD_CHR_TCGETSIZE => self.tcgetsize(trans),
            DCMD_CHR_TCSETSIZE => self.tcsetsize(payload, trans),
            DCMD_CHR_TTYINFO => self.ttyinfo(trans),
            _ => { println!("devctl {:x}", dcmd); Err(libc::ENOSYS) }
        }
    }

    /// Get file descripor I/O flags.
    fn getflags(&mut self, access: u8, trans: &mut Transaction) -> Result<(), i32> {
        let oflag = {
            if access == ACCESS_READ {
                libc::O_RDONLY
            } else if access == ACCESS_WRITE {
                libc::O_WRONLY
            } else if access == ACCESS_READ | ACCESS_WRITE {
                libc::O_RDWR
            } else {
                0
            }
        };

        let reply = GetflagsReply {
            hdr: DevctlReply::new::<i32>(0),
            oflag
        };
        trans.reply_with_payload(0, &reply); Ok(())
    }

    /// Set file descripor I/O flags.
    fn setflags(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        let flags = match transmute_payload::<u32>(payload) {
            Ok(flags) => flags,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        println!("setflags {:o}", flags);
        trans.reply(0);
        Ok(())
    }

    /// Set the session ID for the controlling terminal.
    /// Only one session at a time is allowed to attach to this device.
    fn tcsetsid(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        let sid = match transmute_payload::<i32>(payload) {
            Ok(sid) => *sid,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        if self.job.is_some() {
            return Err(libc::EPERM);
        }

        // FIXME:
        // Permissions.

        // Both the session ID and the foreground process group are set to the same ID.
        let (sess_fd_pid, sess_fd_coid) = trans.connection_id();
        self.job = Some( JobControl { sid, fpgid: sid, sess_fd_pid, sess_fd_coid });
        println!("New session ID {sid}");
        trans.reply(0);
        Ok(())
    }

    /// Set the foreground group for the session.
    fn tcsetpgrp(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        let pgid = match transmute_payload::<i32>(payload) {
            Ok(pgid) => *pgid,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        // Make sure the caller is in the session associated with this device.
        let info = trans.get_client_info()?;
        if !self.sid_match(info.sid) {
            return Err(libc::ENOTTY);
        }

        // Send SIGTTOU signal if client is in a background process group.
        if (info.flags & libc::_NTO_CI_BKGND_PGRP) != 0 {
            let (pid, tid) = (trans.info.pid, trans.info.tid);
            self.signal_background_group(pid, tid, libc::SIGTTOU)?;
        }

        // Let the process manager know about the new foreground group.
        procmgr_session_ext(info.sid, 0, pgid, libc_missing::PROCMGR_SESSION_SETPGRP)?;

        self.job.as_mut().unwrap().fpgid = pgid;
        trans.reply(0);
        Ok(())
    }

    /// Get the ID of the foreground process group.
    fn tcgetpgrp(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        // Make sure the caller is in the session associated with this device.
        let sid = trans.get_client_info()?.sid;
        if !self.sid_match(sid) {
            return Err(libc::ENOTTY);
        }

        match &self.job {
            Some(job) => {
                let reply = TcgetprpReply {
                    hdr: DevctlReply::new::<i32>(0),
                    pgid: job.fpgid
                };
                trans.reply_with_payload(0, &reply); Ok(())
            },
            None => Err(libc::ENOTTY)
        }
    }

    /// Get tarminal attributes.
    fn tcgetattr(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        // These values are based on those provided by devc-con-hid.
        let attr = self.attr.as_termios();

        let reply = TcgetattrReply {
            hdr: DevctlReply::new::<libc::termios>(0),
            attr
        };
        trans.reply_with_payload(0, &reply);
        Ok(())
    }

    /// Set treminal attributes.
    fn tcsetattr(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        // Make sure the caller is in the session associated with this device.
        let sid = trans.get_client_info()?.sid;
        if !self.sid_match(sid) {
            return Err(libc::ENOTTY);
        }

        let attr = match transmute_payload::<libc::termios>(payload) {
            Ok(attr) => attr,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        // Apply new attributes.
        self.attr.input = attr.c_iflag;
        self.attr.output = attr.c_oflag;
        self.attr.control = attr.c_cflag;
        self.attr.local = attr.c_lflag;

        // Apply changes to the display.
        self.display.set_cr_to_nl((attr.c_iflag & libc::ICRNL) != 0);

        trans.reply(0);
        Ok(())
    }

    /// Get screen size.
    fn tcgetsize(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        let (columns, rows) = self.display.screen_size();
        let (width, height) = self.display.framebuffer_size();
        let reply = TcgetsizeReply {
            hdr: DevctlReply::new::<libc::winsize>(0),
            size: libc::winsize {
                ws_col: columns as u16,
                ws_row: rows as u16,
                ws_xpixel: width as u16,
                ws_ypixel: height as u16,
            }
        };
        trans.reply_with_payload(0, &reply); Ok(())
    }

    /// Set the window size.
    fn tcsetsize(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        let winsize = match transmute_payload::<libc::winsize>(payload) {
            Ok(winsize) => winsize,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        let (columns, rows) = self.display.screen_size();
        let (width, height) = self.display.framebuffer_size();
        if (columns != winsize.ws_col as u32) ||
           (rows != winsize.ws_row as u32) ||
           (width != winsize.ws_xpixel as u32) ||
           (height != winsize.ws_ypixel as u32) {
            // No support for changing anything at this point.
            return Err(libc::ENOTSUP);
        }

        trans.reply(0);
        Ok(())
    }

    /// Reply to the ttyinfo request.
    fn ttyinfo(&self, trans: &mut Transaction) -> Result<(), i32> {
        let reply = TtyinfoReply {
            opencount: self.connection_table.count(),
            pad1: 0,
            ttyname: [ '/', 'd', 'e', 'v', '/', 'u', 'e', 'f', 'i' ],
            pad2: [0; 23]
        };

        trans.reply_with_payload(0, &reply);
        Ok(())
    }
}
