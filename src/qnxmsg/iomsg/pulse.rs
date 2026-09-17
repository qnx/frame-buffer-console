use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::Pulse;

pub fn pulse<ServerT>(server: &mut ServerT, buf: &MsgBuf)
where ServerT: IoMsgImpl
{
    let pulse = match buf.get_data_as::<Pulse>(0) {
        Ok(pulse) => pulse,
        Err(err) => { println!("Failed to parse pulse {err} {}", buf.get_length()); return; }
    };

    server.pulse(pulse);
}
