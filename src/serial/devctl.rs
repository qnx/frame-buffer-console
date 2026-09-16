use crate::serial::SerialServer;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::iomsg::DevctlReply;
use crate::libc_missing;

const DCMD_CHR_TCSETSID: u32 = 0x80040308;
const DCMD_CHR_ISATTY: u32 = 0x318;
const DCMD_CHR_TCSETPGRP: u32 = 0x80047476;
const DCMD_CHR_TCGETPGRP: u32 = 0x40047477;
const DCMD_CHR_TCGETATTR : u32 = 0x404c7413;
const DCMD_CHR_TCGETSIZE : u32 = 0x40087468;

const EDIT_CFLAG: u32 = libc::CREAD | libc::CS8 | libc::HUPCL | libc::CLOCAL;
const EDIT_IFLAG: u32 = libc::IMAXBEL | libc::INPCK | libc::BRKINT | libc::ICRNL | libc::IXON |
                        libc::IXOFF;
const EDIT_LFLAG: u32 = libc::ECHO | libc::ECHOCTL | libc::ECHOE | libc::ECHOK | libc::ECHONL |
                        libc::ECHOKE | libc::ICANON | libc::ISIG | libc::IEXTEN;
const EDIT_OFLAG: u32 = libc::ONLCR | libc::OPOST;

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
    pub fn devctl(&mut self, dcmd: u32, payload: &[u8], trans: &mut Transaction) ->
        Result<(), i32> {
        match dcmd {
            DCMD_CHR_ISATTY => { trans.reply(0); Ok(()) },
            DCMD_CHR_TCSETSID => self.tcsetsid(payload, trans),
            DCMD_CHR_TCSETPGRP => self.tcsetpgrp(payload, trans),
            DCMD_CHR_TCGETPGRP => self.tcgetpgrp(trans),
            DCMD_CHR_TCGETATTR => self.tcgetattr(trans),
            DCMD_CHR_TCGETSIZE => self.tcgetsize(trans),
            _ => { println!("devctl {:x}", dcmd); Err(libc::ENOSYS) }
        }
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
                    hdr: DevctlReply {
                        zero: 0,
                        ret_val: 0,
                        nbytes: size_of::<i32>() as u32,
                        zero2: 0
                    },
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
            hdr: DevctlReply {
                zero: 0,
                ret_val: 0,
                nbytes: size_of::<libc::termios>() as u32,
                zero2: 0
            },
            attr
        };
        trans.reply_with_payload(0, &reply); Ok(())
    }

    /// Get screen size.
    fn tcgetsize(&mut self, trans: &mut Transaction) -> Result<(), i32> {
        let (columns, rows) = self.vga.screen_size();
        let (width, height) = self.vga.framebuffer_size();
        let reply = TcgetsizeReply {
            hdr: DevctlReply {
                zero: 0,
                ret_val: 0,
                nbytes: size_of::<libc::termios>() as u32,
                zero2: 0
            },
            size: libc::winsize {
                ws_col: columns as u16,
                ws_row: rows as u16,
                ws_xpixel: width as u16,
                ws_ypixel: height as u16,
            }
        };
        trans.reply_with_payload(0, &reply); Ok(())
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
