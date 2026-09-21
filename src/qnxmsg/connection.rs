use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

/// Hash table for storing connection objects.
pub struct ConnectionTable<ConnectionT> {
    table: HashMap<(i64, i32), Rc<RefCell<ConnectionT>>>
}

impl<ConnectionT> ConnectionTable<ConnectionT> {
    /// Create an empty connection table.
    pub fn new() -> Self {
        ConnectionTable { table: HashMap::<(i64, i32), Rc<RefCell<ConnectionT>>>::new() }
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
    }

    pub fn remove(&mut self, pid_coid: (i64, i32)) -> Option<Rc<RefCell<ConnectionT>>> {
        self.table.remove(&pid_coid)
    }
}
