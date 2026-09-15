#![allow(dead_code)]

use libc;

pub mod mount;
pub mod node;
pub mod iomsg;
pub mod transaction;
pub mod readdir;
pub mod message_loop;
pub mod session;
pub mod channel;
mod msgbuf;

pub const RESMGR_FLAG_DIR: u32 = 0x100;
pub const RESMGR_FLAG_FTYPE_ONLY: u32 = 0x8;

pub const ACCESS_READ: u8 = 0x01;
pub const ACCESS_WRITE: u8 = 0x02;
pub const ACCESS_EXEC: u8 = 0x04;

pub const S_IFDIR: u32 = 0x4000;
pub const S_IFNAM: u32 = 0x5000;
pub const S_IFREG: u32 = 0x8000;
pub const S_INMQ: u32 = 0x3;

#[repr(C, packed)]
#[derive(Default)]
#[derive(Debug)]
pub struct MsgInfo {
    nd: u32,
    srcnd: u32,
    pid: i32,
    tid: i32,
    chid: i32,
    scoid: i32,
    coid: i32,
    priority: i16,
    flags: i16,
    msglen: u64,
    srcmsglen: u64,
    dstmsglen: u64,
    type_id: u32,
    pid_high: i32
}

impl MsgInfo {
    pub fn pid64(&self) -> i64 {
        let high: i64 = self.pid_high.into();
        let low: i64 = self.pid.into();
        (high << 32) | low
    }
}

pub enum FileType {
    ANY,
    FILE,
    LINK,
    SYMLINK,
    PIPE,
    SHMEM,
    MQUEUE,
    SOCKET
}

impl FileType {
    pub fn value(&self) -> u32 {
        match self {
            FileType::ANY => 0,
            FileType::FILE => 1,
            FileType::LINK => 2,
            FileType::SYMLINK => 3,
            FileType::PIPE => 4,
            FileType::SHMEM => 5,
            FileType::MQUEUE => 6,
            FileType::SOCKET => 7
        }
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub struct FilePerms {
    pub uid: i32,
    pub gid: i32,
    pub user_access: u8,
    pub group_access: u8,
    pub world_access: u8,
}

fn access_to_mode(access: u8) -> u32 {
    let mut mode = 0;
    if (access & ACCESS_READ) != 0 {
        mode |= 0o4;
    }
    if (access & ACCESS_WRITE) != 0 {
        mode |= 0o2;
    }
    if (access & ACCESS_EXEC) != 0 {
        mode |= 0o1;
    }
    mode
}

fn mode_to_access(mode: u32) -> u8 {
    let mut access = 0;
    if (mode & 0o4) != 0 {
        access |= ACCESS_READ;
    }
    if (mode & 0o2) != 0 {
        access |= ACCESS_WRITE;
    }
    if (mode & 0o1) != 0 {
        access |= ACCESS_EXEC;
    }
    access
}

impl FilePerms {
    pub fn new(uid: i32, gid: i32, mode: u32) -> Self {
        let user_access = mode_to_access((mode >> 6) & 0x7);
        let group_access = mode_to_access((mode >> 3) & 0x7);
        let world_access = mode_to_access(mode & 0x7);
        FilePerms { uid, gid, user_access, group_access, world_access }
    }

    /// Convert the user/group/world access values into a UNIX mode triplet.
    pub fn get_mode(&self) -> u32 {
        let user_mode = access_to_mode(self.user_access);
        let group_mode = access_to_mode(self.group_access);
        let world_mode = access_to_mode(self.world_access);
        (user_mode << 6) | (group_mode << 3) | world_mode
    }
}
