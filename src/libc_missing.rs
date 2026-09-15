/// Export symbols from the C library not included in the standard crate.

use libc;

pub const HWI_NULL_OFF: u16 = 0xffff;
pub const PROT_NOCACHE: libc::c_int = 0x0000_0800;
pub const MAP_PHYS: libc::c_int = 0x0001_0000;
pub const _NTO_DUPFD_VERIFY: libc::c_int = 5;


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
}
