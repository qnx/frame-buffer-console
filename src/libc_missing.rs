/// Export symbols from the C library not included in the standard crate.

use libc;

pub const HWI_NULL_OFF: u16 = 0xffff;
pub const PROT_NOCACHE: libc::c_int = 0x0000_0800;
pub const MAP_PHYS: libc::c_int = 0x0001_0000;

unsafe extern "C" {
    pub fn hwi_find_tag(start: libc::c_uint, curr_item: libc::c_int, name: *const libc::c_char) ->
        libc::c_uint;
    pub fn hwi_off2tag(off: libc::c_uint) -> *const libc::c_void;
}
