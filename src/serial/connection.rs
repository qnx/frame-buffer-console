use libc;
use crate::qnxmsg::channel::Channel;

pub struct SerialConnection {
    channel: Channel
}

impl SerialConnection {
    pub fn new(channel: Channel) -> Self {
        SerialConnection { channel }
    }

    pub fn emit_key_event(&self, key: u8) -> Result<(), i32> {
        let chid = self.channel.chid;
        let err = unsafe {
            libc::MsgSendPulse_r(-chid, -1, 0, key as i64)
        };

        if err != 0 {
            return Err(err);
        }

        Ok(())
    }
}
