// Copyright (c) 2026 BlackBerry Limited.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::qnxmsg::iomsg::IoMsgImpl;
use crate::qnxmsg::transaction::Transaction;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::iomsg::MsgMsg;

/// Top-level handler for a _IO_MSG message.
pub fn msg<ServerT>(server: &mut ServerT, buf: &MsgBuf,
                      trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    let msg_msg = match buf.get_data_as::<MsgMsg>(0) {
        Ok(msg) => msg,
        Err(err) => { trans.error(err); return; }
    };

    let payload_start = std::mem::size_of::<MsgMsg>();
    let payload_end = buf.get_length();
    let payload = match buf.get_data(payload_start, payload_end) {
        Ok(payload) => payload,
        Err(err) => { trans.error(err); return; }
    };

    if let Err(err) = server.msg(msg_msg, &payload, trans) {
        trans.error(err);
    }
}
