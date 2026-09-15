use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::OpenfdMsg;

/// Top-level handler for a _IO_STAT message.
pub fn openfd<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                       trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let openfd_msg = match buf.get_data_as::<OpenfdMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.openfd(openfd_msg, trans) {
        trans.error(err);
    }
}
