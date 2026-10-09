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

/// Export symbols from the C library not included in the standard crate.

use libc;

pub const HWI_NULL_OFF: u16 = 0xffff;
pub const SHMCTL_PHYS: libc::c_int = 2;
pub const _NTO_DUPFD_VERIFY: libc::c_int = 5;
pub const PROCMGR_SESSION_SETPGRP: u32 = 2;
pub const PROCMGR_SESSION_SIGNAL_PGRP: u32 = 4;
pub const PROCMGR_SESSION_SIGNAL_PID: u32 = 5;
pub const _NOTIFY_ACTION_POLL: u32 = 0x2;
pub const _NOTIFY_ACTION_POLLARM: u32 = 0x3;
pub const _NOTIFY_COND_INPUT: u32 = 0x10000000;
pub const _NOTIFY_COND_OUTPUT: u32 = 0x20000000;
pub const _NOTIFY_COND_OBAND: u32 = 0x40000000;
pub const _CS_HOSTNAME: i32 = 2;
pub const _CS_RELEASE: i32 = 3;
pub const _CS_MACHINE: i32 = 5;

#[cfg(target_arch = "x86_64")]
pub const X86_64_PTE_MTYPE_WC: libc::c_int = 0x10;

#[cfg(target_arch = "aarch64")]
pub const AARCH64_PTE_SODEV: libc::c_int = 0x1;

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

    pub fn rsrcdbmgr_devno_attach(name: *const libc::c_char, minor_request: libc::c_int,
                                  flags: libc::c_int) -> libc::dev_t;

    pub fn shm_ctl_special(fd: libc::c_int, flags: libc::c_int, paddr: u64, size: u64,
                           special: u32) -> i32;

    // The following are in the C library crate, but use the wrong type for receive IDs.
    pub fn MsgReceive_r(chid: i32, msg: *mut libc::c_void, bytes: usize,
                        info: *mut libc::_msg_info64) -> i64;
    pub fn MsgReply(rcvid: i64, status: i64, msg: *const libc::c_void, bytes: usize) -> i32;
    pub fn MsgError(rcvid: i64, err: i32) -> i32;
    pub fn MsgRead_r(rcvid: i64, msg: *mut libc::c_void, bytes: usize, offset: usize) -> i64;
    pub fn MsgDeliverEvent_r(rcvid: i64, event: *const libc::sigevent) -> i32;
    pub fn MsgSendPulse_r(coid: i32, priority: i32, code: i32, value: i64)
                          -> i32;
}
