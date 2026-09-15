use crate::qnxmsg::*;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::transaction::Transaction;

pub mod close;
pub mod connect;
pub mod devctl;
pub mod dup;
pub mod msg;
pub mod openfd;
pub mod read;
pub mod stat;
pub mod write;

pub const _IO_CONNECT: u16 = 0x100;
pub const _IO_CONNECT_COMBINE: u16 = 0;
pub const _IO_CONNECT_COMBINE_CLOSE: u16 = 1;
pub const _IO_CONNECT_OPEN: u16 = 2;
pub const _IO_CONNECT_UNLINK: u16 = 3;
pub const _IO_CONNECT_RENAME: u16 = 4;
pub const _IO_CONNECT_MKMOD: u16 = 5;
pub const _IO_CONNECT_READLINK: u16 = 6;
pub const _IO_CONNECT_LINK: u16 = 7;
pub const _IO_CONNECT_MOUNT: u16 = 9;

pub const _IO_CONNECT_EXTRA_NONE: u8 = 0;
pub const _IO_CONNECT_EXTRA_LINK_RESERVED: u8 = 1;
pub const _IO_CONNECT_EXTRA_SYMLINK: u8 = 2;
pub const _IO_CONNECT_EXTRA_MQUEUE: u8 = 3;
pub const _IO_CONNECT_EXTRA_PHOTON: u8 = 4;
pub const _IO_CONNECT_EXTRA_SOCKET: u8 = 5;
pub const _IO_CONNECT_EXTRA_SEM: u8 = 6;
pub const _IO_CONNECT_EXTRA_RESMGR_LINK: u8 = 7;
pub const _IO_CONNECT_EXTRA_PROC_SYMLINK: u8 = 8;
pub const _IO_CONNECT_EXTRA_RENAME: u8 = 9;
pub const _IO_CONNECT_EXTRA_MOUNT: u8 = 10;
pub const _IO_CONNECT_EXTRA_MOUNT_OCB: u8 = 11;
pub const _IO_CONNECT_EXTRA_TYMEM: u8 = 12;
pub const _IO_CONNECT_EXTRA_LINK: u8 = 13;

pub const _IO_READ: u16 = 0x101;
pub const _IO_WRITE: u16 = 0x102;
pub const _IO_STAT: u16 = 0x104;
pub const _IO_DEVCTL: u16 = 0x106;
pub const _IO_OPENFD: u16 = 0x10d;
pub const _IO_MSG: u16 = 0x113;
pub const _IO_DUP: u16 = 0x115;
pub const _IO_CLOSE: u16 = 0x116;

#[repr(C, packed)]
#[derive(Debug)]
pub struct ConnectMsg {
    msg_type: u16,
    pub subtype: u16,
    pub file_type: u32,
    reply_max: u16,
    entry_max: u16,
    reserved: u32,
    handle: u32,
    pub ioflag: u32,
    pub mode: u32,
    sflag: u16,
    access: u16,
    dirfd: u16,
    path_len: u16,
    eflag: u8,
    extra_type: u8,
    extra_len: u16,
}

/// An object representing the path requested by a connect message.
pub struct ConnectPath {
    pub path: String,
    pub path_end: usize
}

/// Encapsulates the optional extra data passed to a connect message.
pub struct ConnectExtra<'a> {
    extra_type: u8,
    pub start: usize,
    pub end: usize,
    buf: &'a MsgBuf
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct ReadMsg {
    msg_type: u16,
    combine_len: u16,
    pub nbytes: u32,
    pub xtype: u32,
    pub zero: u32
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct WriteMsg {
    msg_type: u16,
    combine_len: u16,
    pub nbytes: u32,
    pub xtype: u32,
    pub zero: u32
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct StatMsg {
    msg_type: u16,
    combine_len: u16,
    pub format: u32
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct DevctlMsg {
    msg_type: u16,
    combine_len: u16,
    pub dcmd: u32,
    pub nbytes: u32,
    pub zero: u32
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct DupMsg {
    msg_type: u16,
    combine_len: u16,
    nd: u32,
    srcnd: u32,
    pub pid: i32,
    tid: i32,
    chid: i32,
    pub scoid: i32,
    pub coid: i32,
    pub handle: u64
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct MsgMsg {
    msg_type: u16,
    combine_len: u16,
    pub mgrid: u16,
    pub subtype: u16
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct OpenfdMsg {
    msg_type: u16,
    combine_len: u16,
    pub ioflag: u32,
    pub sflag: u16,
    pub xtype: u16,
    pub info: libc_missing::_msg_info_hdr,
    pub handle: u64
}

/// Trait for I/O messages delivered to the server.
pub trait IoMsgImpl {
    fn connect(&mut self, _msg: &ConnectMsg, _path: &str,
               _extra: &ConnectExtra, _trans: &mut Transaction)
               -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn unlink(&mut self, _msg: &ConnectMsg, _path: &str, _trans: &mut Transaction)
              -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn stat(&mut self, _msg: &StatMsg, _trans: &mut Transaction)
            -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn read(&mut self, _msg: &ReadMsg, _trans: &mut Transaction)
            -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn write(&mut self, _msg: &WriteMsg, _str: &str, _trans: &mut Transaction)
            -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn devctl(&mut self, _msg: &DevctlMsg, _payload: &[u8], _trans: &mut Transaction)
              -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn dup(&mut self, _msg: &DupMsg, _trans: &mut Transaction)
           -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn msg(&mut self, _msg: &MsgMsg, _payload: &[u8], _trans: &mut Transaction)
           -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn openfd(&mut self, _msg: &OpenfdMsg, _trans: &mut Transaction)
           -> Result<(), i32> {
        Err(libc::ENOSYS)
    }

    fn close(&mut self, _trans: &Transaction)
             -> Result<(), i32> {
        Err(libc::ENOSYS)
    }
}
