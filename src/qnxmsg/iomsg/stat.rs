use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::StatMsg;

/// Top-level handler for a _IO_STAT message.
pub fn stat<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                     trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let stat_msg = match buf.get_data_as::<StatMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.stat(stat_msg, trans) {
        trans.error(err);
    }
}
