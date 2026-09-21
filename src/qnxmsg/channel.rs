use crate::qnxmsg::MsgInfo;
use crate::qnxmsg::Pulse;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::libc_missing;

/// Channel object.
#[derive(Clone,Copy)]
pub struct Channel {
    pub chid: i32
}

impl Channel {
    pub fn new(flags: u32) -> Result<Self, i32> {
        let chid =
            unsafe {
                libc::ChannelCreate_r(flags)
            };

        if chid < 0 {
            return Err(-chid);
        }

        let channel = Channel {
            chid
        };

        Ok(channel)
    }

    pub fn receive(&self, buf: &mut MsgBuf, info: &mut MsgInfo) -> Result<i64, i32> {
        let rcvid = unsafe {
            let buf_ptr = buf.as_void_ptr();
            let info_ptr = std::ptr::addr_of!(*info)
                as *mut libc::_msg_info64;

            libc_missing::MsgReceive_r(self.chid, buf_ptr,
                                       std::mem::size_of::<MsgBuf>(),
                                       info_ptr)
        };

        if rcvid < 0 {
            return Err(-rcvid as i32);
        }

        if rcvid == 0 {
            buf.set_length(size_of::<Pulse>());
        } else {
            buf.set_length(info.msglen as usize);
        }
        Ok(rcvid)
    }
}
