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

/// Top-level handler for a _IO_CLOSE message.
pub fn close<ServerT>(server: &mut ServerT, trans: &mut Transaction)
where ServerT: IoMsgImpl
{
    match server.close(trans) {
        Ok(_) => trans.reply_maybe(0),
        Err(err) => trans.error(err)
    }
}
