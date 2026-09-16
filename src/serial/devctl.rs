use crate::serial::SerialServer;
use crate::qnxmsg::*;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::iomsg::DevctlReply;
use crate::libc_missing;

const DCMD_ALL_GETFLAGS: u32 = 0x40040101;
const DCMD_CHR_TCSETSID: u32 = 0x80040308;
const DCMD_CHR_ISATTY: u32 = 0x318;
const DCMD_CHR_TCSETPGRP: u32 = 0x80047476;
const DCMD_CHR_TCGETPGRP: u32 = 0x40047477;
const DCMD_CHR_TCGETATTR : u32 = 0x404c7413;
const DCMD_CHR_TCSETATTRD : u32 = 0x804c7415;
const DCMD_CHR_TCGETSIZE : u32 = 0x40087468;
const DCMD_CHR_TCSETSIZE : u32 = 0x80087467;

const EDIT_CFLAG: u32 = libc::CREAD | libc::CS8 | libc::HUPCL | libc::CLOCAL;
const EDIT_IFLAG: u32 = libc::IMAXBEL | libc::INPCK | libc::BRKINT | libc::ICRNL | libc::IXON |
                        libc::IXOFF;
const EDIT_LFLAG: u32 = libc::ECHO | libc::ECHOCTL | libc::ECHOE | libc::ECHOK | libc::ECHONL |
                        libc::ECHOKE | libc::ICANON | libc::ISIG | libc::IEXTEN;
const EDIT_OFLAG: u32 = libc::ONLCR | libc::OPOST;

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
        let req_access = match dcmd {
            DCMD_ALL_GETFLAGS => ACCESS_READ,
            DCMD_CHR_ISATTY => ACCESS_READ,
            DCMD_CHR_TCSETSID => ACCESS_WRITE,
            DCMD_CHR_TCSETPGRP => ACCESS_WRITE,
            DCMD_CHR_TCGETPGRP => ACCESS_READ,
            DCMD_CHR_TCGETATTR => ACCESS_READ,
            DCMD_CHR_TCSETATTRD => ACCESS_WRITE,
            DCMD_CHR_TCGETSIZE => ACCESS_READ,
            DCMD_CHR_TCSETSIZE => ACCESS_WRITE,
            _ => ACCESS_WRITE
        };

        if (req_access & access) == 0 {
            return Err(libc::EPERM);
        }

        match dcmd {
            DCMD_ALL_GETFLAGS => self.getflags(access, trans),
            DCMD_CHR_ISATTY => { trans.reply(0); Ok(()) },
            DCMD_CHR_TCSETSID => self.tcsetsid(payload, trans),
            DCMD_CHR_TCSETPGRP => self.tcsetpgrp(payload, trans),
            DCMD_CHR_TCGETPGRP => self.tcgetpgrp(trans),
            DCMD_CHR_TCGETATTR => self.tcgetattr(trans),
            DCMD_CHR_TCSETATTRD => self.tcsetattr(payload, trans),
            DCMD_CHR_TCGETSIZE => self.tcgetsize(trans),
            DCMD_CHR_TCSETSIZE => self.tcsetsize(payload, trans),
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

    /// Set the session ID for the controlling terminal.
    /// Only one session at a time is allowed to attach to this device.
    fn tcsetsid(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        let sid = match transmute_payload::<i32>(payload) {
            Ok(sid) => *sid,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        if self.sid.is_some() {
            return Err(libc::EPERM);
        }

        // FIXME:
        // Permissions.

        // Both the session ID and the foreground process group are set to the same ID.
        self.sid = Some(sid);
        self.fpgid = Some(sid);
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
        let sid = trans.get_client_info()?.sid;
        if Some(sid) != self.sid {
            return Err(libc::ENOTTY);
        }

        // FIXME:
        // Send SIGTTOU signal?

        // Let the process manager know about the new foreground group.
        procmgr_session_ext(sid, 0, pgid, libc_missing::PROCMGR_SESSION_SETPGRP)?;

        self.fpgid = Some(pgid);
        println!("New foreground process groupd {pgid}");
        trans.reply(0);
        Ok(())
    }

    /// Get the ID of the foreground process group.
    fn tcgetpgrp(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        // Make sure the caller is in the session associated with this device.
        let sid = trans.get_client_info()?.sid;
        if Some(sid) != self.sid {
            return Err(libc::ENOTTY);
        }

        match self.fpgid {
            Some(pgid) => {
                let reply = TcgetprpReply {
                    hdr: DevctlReply::new::<i32>(0),
                    pgid
                };
                trans.reply_with_payload(0, &reply); Ok(())
            },
            None => Err(libc::ENOTTY)
        }
    }

    /// Get tarminal attributes.
    fn tcgetattr(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        // These values are based on those provided by devc-con-hid.
        let mut attr: libc::termios = unsafe { std::mem::zeroed() };
        attr.c_iflag = EDIT_IFLAG;
        attr.c_oflag = EDIT_OFLAG;
        attr.c_cflag = EDIT_CFLAG;
        attr.c_lflag = EDIT_LFLAG;
        attr.c_ispeed = 100000;
        attr.c_ospeed = 100000;

        let reply = TcgetattrReply {
            hdr: DevctlReply::new::<libc::termios>(0),
            attr
        };
        trans.reply_with_payload(0, &reply); Ok(())
    }

    /// Set treminal attributes.
    fn tcsetattr(&mut self, payload: &[u8], trans: &mut Transaction) -> Result<(), i32> {
        // Make sure the caller is in the session associated with this device.
        let sid = trans.get_client_info()?.sid;
        if Some(sid) != self.sid {
            return Err(libc::ENOTTY);
        }

        let _attr = match transmute_payload::<libc::termios>(payload) {
            Ok(attr) => attr,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        // FIXME:
        // Apply attributes.

        trans.reply(0);
        Ok(())
    }

    /// Get screen size.
    fn tcgetsize(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        let (columns, rows) = self.vga.screen_size();
        let (width, height) = self.vga.framebuffer_size();
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

        let (columns, rows) = self.vga.screen_size();
        let (width, height) = self.vga.framebuffer_size();
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
}

fn procmgr_session_ext(sid: libc::pid_t, xid: i32, id: i32, event: u32) -> Result<(), i32> {
    let err = unsafe {
        libc_missing::procmgr_session_ext(0, sid, xid, id, event)
    };

    if err != 0 {
        return Err(err);
    }

    Ok(())
}
