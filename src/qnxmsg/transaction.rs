use std::mem::MaybeUninit;
use crate::qnxmsg::*;
use crate::qnxmsg::msgbuf::MsgBuf;

/// Message transaction object.
/// Passed to all message handlers, and keeps track of whether the message has
/// been replied to.
pub struct Transaction {
    rcvid: i64,
    state: State,
    combine_offset: usize,
    auto_close: bool,
    pub info: MsgInfo,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum State {
    None,
    Pending,
    Replied,
    Error
}

pub enum CombineMessageStatus {
    More,
    Close,
    Done
}

impl Transaction {
    pub fn new() -> Self {
        Transaction {
            rcvid: -1,
            state: State::None,
            combine_offset: 0,
            auto_close: false,
            info: MsgInfo::default()
        }
    }

    /// Update the receive ID associated with the transaction, as a result of
    /// receiving a new message.
    /// The transaction state must represent either a new object, or a
    /// transaction that has already been terminated (replied to or errored).
    pub fn set_rcvid(&mut self, rcvid: i64) {
        assert!(rcvid > 0);
        assert_ne!(self.state, State::Pending);
        self.rcvid = rcvid;
        self.state = State::Pending;
    }

    pub fn get_rcvid(&self) -> i64 {
        self.rcvid
    }

    pub fn get_reply_size(&self) -> usize {
        self.info.dstmsglen as usize
    }

    /// Reply to a message with a status.
    pub fn reply(&mut self, status: i64) {
        assert_eq!(self.state, State::Pending);
        self.reply_common(status, std::ptr::null_mut(), 0);
    }

    /// Reply to a message if it has not already been replied to.
    /// Used from the close handler to handle the case of a connect/combine/close
    /// message that was already replied to.
    pub fn reply_maybe(&mut self, status: i64) {
        if self.state == State::Pending {
            self.reply_common(status, std::ptr::null_mut(), 0);
        }
    }

    /// Reply to a message with a status and payload.
    pub fn reply_with_payload<T>(&mut self, status: i64, payload: &T) {
        assert_eq!(self.state, State::Pending);
        let payload_ptr = std::ptr::addr_of!(*payload) as *mut libc::c_void;
        self.reply_common(status, payload_ptr, size_of::<T>());
    }

    /// Reply to a message with a status and payload.
    pub fn reply_with_payload_and_size<T: ?Sized>(&mut self, status: i64,
                                                  payload: &T, size: usize) {
        assert_eq!(self.state, State::Pending);
        let payload_ptr = std::ptr::addr_of!(*payload) as *mut libc::c_void;
        self.reply_common(status, payload_ptr, size);
    }

    /// Common functionality to all reply variants.
    fn reply_common(&mut self, status: i64, ptr: *mut libc::c_void,
                    size: usize) {
        // Invoke the kernel call.
        unsafe {
            libc_missing::MsgReply(self.rcvid, status, ptr, size);
        };

        // Update transaction state.
        self.state = State::Replied;
    }

    /// Reply to a message with an error.
    pub fn error(&mut self, error: i32) {
        assert_eq!(self.state, State::Pending);

        unsafe {
            libc_missing::MsgError(self.rcvid, error);
        };

        self.state = State::Error;
    }

    /// Set the transaction state to indicate a combine message.
    /// The transaction must be in the Pending state.
    pub fn set_combine(&mut self, off: usize, auto_close: bool) {
        assert_eq!(self.state, State::Pending);
        self.combine_offset = off;
        self.auto_close = auto_close;
    }

    /// Update a transaction to reflect the next message in a combine message.
    pub fn get_combine_message(&mut self, buf: &mut MsgBuf) ->
        CombineMessageStatus {
        let off = self.combine_offset;
        if off == 0 {
            let status = if self.auto_close {
                self.auto_close = false;
                CombineMessageStatus::Close
            } else {
                CombineMessageStatus::Done
            };

            return status;
        }

        // Next message is at the given offset.
        buf.shift(off);
        self.combine_offset = 0;
        CombineMessageStatus::More
    }

    /// Get the client's user and group IDs.
    pub fn get_client_info(&self) -> Result<libc::_client_info, i32> {
        // FIXME:
        // Get all supplementary groups?
        let mut info = MaybeUninit::<libc::_client_info>::uninit();
        let err = unsafe {
            let info_ptr = info.as_mut_ptr() as *mut libc::_client_info;
            libc::ConnectClientInfo_r(self.info.scoid, info_ptr, 8)
        };

        if err < 0 {
            return Err(-err);
        }

        let info = unsafe { info.assume_init() };
        Ok(info)
    }

    /// Perform permission checks on the client's credentials.
    pub fn check_credentials(&self, perms: &FilePerms,
                             req_access: u8) -> bool {
        // Get client credentials.
        let info = match self.get_client_info() {
            Err(_err) => { return false; },
            Ok(cred) => cred
        };

        if (perms.uid == info.cred.ruid) || (perms.uid == info.cred.euid) {
            // Check by user permissions.
            if (perms.user_access & req_access) == req_access {
                return true;
            }
        }

        if (perms.gid == info.cred.rgid) || (perms.gid == info.cred.egid) {
            // Check by group permissions.
            if (perms.group_access & req_access) == req_access {
                return true;
            }
        }

        // Check by world permissions.
        if (perms.world_access & req_access) == req_access {
            return true;
        }

        false
    }

    pub fn connection_id(&self) -> (i64, i32) {
        (self.info.pid64(), self.info.coid)
    }

    pub fn reply_later(&mut self) -> Transaction {
        // Mark the existing transaction to prevent replies.
        self.state = State::Error;

        // Create a copy of the transaction object.
        Transaction {
            rcvid: self.rcvid,
            state: State::Pending,
            combine_offset: 0,
            auto_close: false,
            info: self.info
        }
    }

    /// Read message contents at the given offset.
    /// The function is typically used when a message is larger than the buffer supplied to
    /// Channel::receive() to access the remaining bytes.
    pub fn read(&self, data: &mut [u8], offset: usize) -> Result<usize, i32> {
        let rc = unsafe {
            libc_missing::MsgRead_r(self.rcvid, data as *mut [u8] as *mut libc::c_void,
                                    data.len(), offset)
        };

        if rc < 0 {
            return Err(-rc as i32);
        }

        Ok(rc as usize)
    }
}
