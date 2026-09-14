use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::DupMsg;

/// Top-level handler for a _IO_DUP message.
pub fn dup<ServerT>(server: &mut ServerT, buf: &MsgBuf, trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let dup_msg = match buf.get_data_as::<DupMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.dup(dup_msg, trans) {
        trans.error(err);
    }
}
