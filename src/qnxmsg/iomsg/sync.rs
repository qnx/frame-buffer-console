use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::SyncMsg;

/// Top-level handler for a _IO_SYNC message.
pub fn sync<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                      trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let sync_msg = match buf.get_data_as::<SyncMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.sync(sync_msg, trans) {
        trans.error(err);
    }
}
