/// Export symbols from the C library not included in the standard crate.

use libc;

pub const HWI_NULL_OFF: u16 = 0xffff;
pub const PROT_NOCACHE: libc::c_int = 0x0000_0800;
pub const MAP_PHYS: libc::c_int = 0x0001_0000;
pub const _NTO_DUPFD_VERIFY: libc::c_int = 5;
pub const PROCMGR_SESSION_SETPGRP: u32 = 2;
pub const _NOTIFY_ACTION_POLL: u32 = 0x2;
pub const _NOTIFY_ACTION_POLLARM: u32 = 0x3;
pub const _NOTIFY_COND_INPUT: u32 = 0x10000000;
pub const _NOTIFY_COND_OUTPUT: u32 = 0x20000000;
pub const _NOTIFY_COND_OBAND: u32 = 0x40000000;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct _msg_info_hdr {
    nd: u32,
    srcnd: u32,
    pid: i32,
    tid: i32,
    chid: i32,
    scoid: i32,
    coid: i32,
}

unsafe extern "C" {
    pub fn hwi_find_tag(start: libc::c_uint, curr_item: libc::c_int, name: *const libc::c_char) ->
        libc::c_uint;
    pub fn hwi_off2tag(off: libc::c_uint) -> *const libc::c_void;

    pub fn ConnectDupFd_r(op: libc::c_int, dstpid: *const i64, handle: *const u64, srvpid: *mut i32,
                          info: *const _msg_info_hdr) -> i32;

    pub fn procmgr_session_ext(reserved: u32, sid: libc::pid_t, xid: i32, id: i32, event: u32) ->
        i32;
}
