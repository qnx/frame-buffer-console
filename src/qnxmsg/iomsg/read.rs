use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::ReadMsg;

/// Top-level handler for a _IO_READ message.
pub fn read<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                     trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let read_msg = match buf.get_data_as::<ReadMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.read(read_msg, trans) {
        trans.error(err);
    }
}
