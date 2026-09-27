use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::WriteMsg;

/// Top-level handler for a _IO_WRITE message.
pub fn write<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                      trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    // Get the message's header.
    let write_msg = match buf.get_data_as::<WriteMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    // Determine the payload (the bytes written by the client.
    // Accept messages where the payload size, as reported by WriteMsg.nbytes, is smaller than the
    // number of bytes available, but not the other way around.
    let payload_start = std::mem::size_of::<WriteMsg>();
    let mut payload_end = payload_start + write_msg.nbytes as usize;
    if payload_end > trans.info.srcmsglen as usize {
        trans.error(libc::EBADMSG);
        return;
    }

    // The message may be longer than the receive buffer.
    if payload_end > buf.get_length() {
        payload_end = buf.get_length();
    }

    let payload = match buf.get_data(payload_start, payload_end) {
        Ok(payload) => payload,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.write(write_msg, payload, trans) {
        trans.error(err);
    }
}
