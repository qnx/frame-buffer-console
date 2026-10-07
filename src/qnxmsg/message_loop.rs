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

use crate::qnxmsg::channel::Channel;
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::transaction::*;
use crate::qnxmsg::iomsg::*;
use crate::qnxmsg::iomsg::close::*;
use crate::qnxmsg::iomsg::connect::*;
use crate::qnxmsg::iomsg::devctl::*;
use crate::qnxmsg::iomsg::dup::*;
use crate::qnxmsg::iomsg::msg::*;
use crate::qnxmsg::iomsg::notify::*;
use crate::qnxmsg::iomsg::openfd::*;
use crate::qnxmsg::iomsg::read::*;
use crate::qnxmsg::iomsg::stat::*;
use crate::qnxmsg::iomsg::sync::*;
use crate::qnxmsg::iomsg::write::*;
use crate::qnxmsg::iomsg::pulse::*;

pub fn message_loop<ServerT>(server: &mut ServerT, channel: Channel)
where ServerT: IoMsgImpl
{
    // FIXME:
    // Revisit how buf and info are allocated.
    let mut buf = MsgBuf::new();
    loop {
        match receive(server, channel, &mut buf) {
            Ok(()) => {},
            Err(libc::EINTR) => {},
            Err(err) => panic!("Recive failed: {err}")
        };
    }
}

/// Receive a message or a pulse on the channel.
fn receive<ServerT>(server: &mut ServerT, channel: Channel, buf: &mut MsgBuf)
                    -> Result<(), i32>
where ServerT: IoMsgImpl
{
    // Receive a single message/pulse.
    let mut trans = Transaction::new();
    let rcvid = channel.receive(buf, &mut trans.info)?;

    if rcvid == 0 {
        // Received a pulse.
        pulse(server, buf);
        return Ok(());
    }

    trans.set_rcvid(rcvid);

    // Dispatch the message to the appropriate handler.
    // May need to loop in order to handle combine messages.
    loop {
        let msg_type = buf.get_type();
        match msg_type {
            _IO_CONNECT => connect(server, buf, &mut trans),
            _IO_READ => read(server, buf, &mut trans),
            _IO_WRITE => write(server, buf, &mut trans),
            _IO_STAT => stat(server, buf, &mut trans),
            _IO_NOTIFY => notify(server, buf, &mut trans),
            _IO_DEVCTL => devctl(server, buf, &mut trans),
            _IO_OPENFD => openfd(server, buf, &mut trans),
            _IO_MSG => msg(server, buf, &mut trans),
            _IO_DUP => dup(server, buf, &mut trans),
            _IO_SYNC => sync(server, buf, &mut trans),
            _IO_CLOSE => close(server, &mut trans),
            _ => {
                println!("Unhandled message {msg_type}");
                trans.error(libc::ENOSYS);
                break;
            }
        };

        match trans.get_combine_message(buf) {
            CombineMessageStatus::More => { }
            CombineMessageStatus::Close => {
                close(server, &mut trans);
                break;
            }
            CombineMessageStatus::Done => { break; }
        };
    };

    Ok(())
}
