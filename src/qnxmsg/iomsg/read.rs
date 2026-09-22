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

    // Remember the number of bytes the client is expecting in response, if it is smaller than the
    // reply buffer's size.
    let max_reply_size = read_msg.nbytes as u64;
    if max_reply_size < trans.info.dstmsglen {
        trans.info.dstmsglen = max_reply_size;
    }

    if let Err(err) = server.read(read_msg, trans) {
        trans.error(err);
    }
}
