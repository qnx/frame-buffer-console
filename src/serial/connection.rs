use crate::qnxmsg::channel::Channel;
use crate::hid::key::Key;
use crate::libc_missing;

pub struct SerialConnection {
    channel: Channel
}

impl SerialConnection {
    pub fn new(channel: Channel) -> Self {
        SerialConnection { channel }
    }

    pub fn emit_key_event(&self, key: Key) -> Result<(), i32> {
        let chid = self.channel.chid;
        let err = unsafe {
            libc_missing::MsgSendPulse_r(-chid, -1, 0, key.to_pulse_value())
        };

        if err != 0 {
            return Err(err);
        }

        Ok(())
    }
}
