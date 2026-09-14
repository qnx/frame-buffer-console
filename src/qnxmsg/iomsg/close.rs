use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;

/// Top-level handler for a _IO_CLOSE message.
pub fn close<ServerT>(server: &mut ServerT, trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    match server.close(trans) {
        Ok(_) => trans.reply_maybe(0),
        Err(err) => trans.error(err)
    }
}
