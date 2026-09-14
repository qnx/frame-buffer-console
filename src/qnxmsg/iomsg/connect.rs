use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::*;

const MESSAGE_ALIGNMENT: usize = 8;

fn align_msg_offset(offset: usize) -> usize {
    (offset + MESSAGE_ALIGNMENT - 1usize) & !(MESSAGE_ALIGNMENT - 1usize)
}

/// Top-level handler for a _IO_CONNECT message.
pub fn connect<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                        trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    // Convert to ConnectMsg.
    let connect_msg = match buf.get_data_as::<ConnectMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    // Extract path.
    let path = match ConnectPath::new(connect_msg, buf) {
        Err(err) => { trans.error(err); return; }
        Ok(p) => p
    };

    // Get the extra data, if any.
    let extra_start = align_msg_offset(path.path_end);
    let extra = match ConnectExtra::new(connect_msg, extra_start, buf) {
        Err(err) => { trans.error(err); return; }
        Ok(e) => e
    };

    let subtype = connect_msg.subtype;

    // Invoke server-specific implementation.
    let err = match subtype {
        _IO_CONNECT_UNLINK => server.unlink(connect_msg, &path.path, trans),
        _ => server.connect(connect_msg, &path.path, &extra, trans)
    };

    match err {
        Err(err) => { trans.error(err); return },
        Ok(_) => {}
    }

    match subtype {
        _IO_CONNECT_COMBINE => { trans.set_combine(extra.start, false); }
        _IO_CONNECT_COMBINE_CLOSE => { trans.set_combine(extra.start, true); }
        _ => { trans.reply(0); }
    }
}

impl<'a> ConnectExtra<'a> {
    pub fn new(msg: &ConnectMsg, start: usize, buf: &'a MsgBuf)
               -> Result<Self, i32> {
        let len: usize = msg.extra_len.into();
        let end: usize = start + len;

        if (len > 0) && !buf.valid_range(start, end) {
            return Err(libc::EBADMSG);
        }

        Ok(ConnectExtra {
            extra_type: msg.extra_type,
            start,
            end,
            buf
        })
    }

    /// Convert the extra data bytes into a specific type.
    /// The expected type must match the one specified by the _IO_CONNECT
    /// message, and the message should have sufficient bytes for the size of
    /// type.
    pub fn get_data<T>(&self, extra_type: u8) -> Result<Option<&T>, i32> {
        if self.start == self.end {
            return Ok(None);
        }

        if self.extra_type != extra_type {
            return Err(libc::EBADMSG);
        }

        match self.buf.get_data_as::<T>(self.start) {
            Ok(t) => Ok(Some(t)),
            Err(err) => Err(err)
        }
    }
}

impl ConnectPath {
    pub fn new(msg: &ConnectMsg, buf: &MsgBuf) -> Result<Self, i32> {
        // The message contains a NUL-terminated path starting immediately after
        // the fixed part of the message. The path_len member of the message
        // holds the length of the path, and includes the terminating NUL.
        let path_len: usize = msg.path_len.into();
        if path_len == 0 {
            return Err(libc::EBADMSG);
        }

        // Determine the end of the path in the message (including the NUL
        // terminator).
        const CONNECT_PATH_OFFSET: usize = size_of::<ConnectMsg>();
        let path_end: usize = CONNECT_PATH_OFFSET + path_len;

        // Get a view into the buffer. Do not include the NUL terminator, as
        // that is going to mess the conversion to String.
        let path_result = &buf.get_data(CONNECT_PATH_OFFSET, path_end - 1);
        let path = match path_result {
            Ok(path) => path,
            Err(_) => { return Err(libc::EBADMSG); }
        };

        // Convert the message bytes to a String.
        let path = match std::str::from_utf8(path) {
            Ok("") => {
                // Empty path, use /mount_handle as the path.
                let mount_handle = msg.handle;
                format!("/{mount_handle}")
            }
            Ok(s) => s.to_string(),
            Err(_) => { return Err(libc::EBADMSG); }
        };

        Ok(ConnectPath { path, path_end })
    }
}
