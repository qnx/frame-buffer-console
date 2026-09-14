use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

/// Hash table for storing session objects.
pub struct SessionTable<SessionT> {
    table: HashMap<(i64, i32), Rc<RefCell<SessionT>>>
}

impl<SessionT> SessionTable<SessionT> {
    /// Create an empty session table.
    pub fn new() -> Self {
        SessionTable { table: HashMap::<(i64, i32), Rc<RefCell<SessionT>>>::new() }
    }

    /// Find a session object given a process ID and a client connection ID.
    /// The caller takes ownership of the object.
    pub fn get(&self, pid_coid: (i64, i32)) -> Option<&Rc<RefCell<SessionT>>> {
        self.table.get(&pid_coid)
    }

    /// Add a new session to the table.
    /// A session with the same process ID and connection ID must not already
    /// exist.
    pub fn add(&mut self, pid_coid: (i64, i32), session: &Rc<RefCell<SessionT>>) {
        let session = Rc::clone(session);
        let result = self.table.insert(pid_coid, session);
        assert!(result.is_none());
    }

    pub fn remove(&mut self, pid_coid: (i64, i32)) -> Option<Rc<RefCell<SessionT>>> {
        self.table.remove(&pid_coid)
    }
}
