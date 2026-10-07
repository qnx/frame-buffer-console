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

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

/// Hash table for storing connection objects.
pub struct ConnectionTable<ConnectionT> {
    table: HashMap<(i64, i32), Rc<RefCell<ConnectionT>>>,
    count: u32
}

impl<ConnectionT> ConnectionTable<ConnectionT> {
    /// Create an empty connection table.
    pub fn new() -> Self {
        ConnectionTable {
            table: HashMap::<(i64, i32), Rc<RefCell<ConnectionT>>>::new(),
            count: 0
        }
    }

    /// Find a connection object given a process ID and a client connection ID.
    /// The caller takes ownership of the object.
    pub fn get(&self, pid_coid: (i64, i32)) -> Option<&Rc<RefCell<ConnectionT>>> {
        self.table.get(&pid_coid)
    }

    /// Add a new connection to the table.
    /// A connection with the same process ID and connection ID must not already
    /// exist.
    pub fn add(&mut self, pid_coid: (i64, i32), connection: &Rc<RefCell<ConnectionT>>) {
        let connection = Rc::clone(connection);
        let result = self.table.insert(pid_coid, connection);
        assert!(result.is_none());
        self.count += 1;
    }

    pub fn remove(&mut self, pid_coid: (i64, i32)) -> Option<Rc<RefCell<ConnectionT>>> {
        self.count -= 1;
        self.table.remove(&pid_coid)
    }

    /// Return the number of open connections.
    pub fn count(&self) -> u32 {
        self.count
    }
}
