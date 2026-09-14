use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::DevctlMsg;

/// Top-level handler for a _IO_STAT message.
pub fn devctl<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                       trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let devctl_msg = match buf.get_data_as::<DevctlMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.devctl(devctl_msg, trans) {
        trans.error(err);
    }
}
