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
use crate::qnxmsg::msgbuf::MsgBuf;
use crate::qnxmsg::Pulse;

pub fn pulse<ServerT>(server: &mut ServerT, buf: &MsgBuf)
where ServerT: IoMsgImpl
{
    let pulse = match buf.get_data_as::<Pulse>(0) {
        Ok(pulse) => pulse,
        Err(err) => { println!("Failed to parse pulse {err} {}", buf.get_length()); return; }
    };

    server.pulse(pulse);
}
