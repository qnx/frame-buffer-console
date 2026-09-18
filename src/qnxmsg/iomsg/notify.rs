use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::NotifyMsg;

/// Top-level handler for a _IO_WRITE message.
pub fn notify<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                      trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let notify_msg = match buf.get_data_as::<NotifyMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    // The payload is an array of struct pollfd.
    let payload_start = std::mem::size_of::<NotifyMsg>();
    let fds = match buf.get_data_as_slice_of::<libc::pollfd>(payload_start) {
        Ok(fds) => fds,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.notify(notify_msg, fds, trans) {
        trans.error(err);
    }
}
