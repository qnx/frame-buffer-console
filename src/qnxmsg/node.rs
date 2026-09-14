use std::rc::Rc;
use std::collections::HashMap;
use std::collections::hash_map::Iter;

/// Node table associating server-specific node objects with paths.
/// Attach-point nodes correspond to paths of the form '/handle', where 'handle'
/// is a 32-bit value passed to Server.attach().
/// Nodes under an attach point are represented by relative paths, i.e., paths
/// not starting with a forward slash.
pub struct NodeTable<NodeT> {
    table: HashMap<String, Rc<NodeT>>,
}

pub type NodeTableIter<'a, NodeT> = Iter<'a, String, Rc<NodeT>>;

impl<NodeT> NodeTable<NodeT> {
    pub fn new() -> Self {
        NodeTable { table: HashMap::<String, Rc<NodeT>>::new() }
    }

    pub fn get(&self, path: &str) -> Option<&Rc<NodeT>> {
        self.table.get(path)
    }

    pub fn add(&mut self, path: &str, node: &Rc<NodeT>) -> Option<&Rc<NodeT>> {
        let node = Rc::clone(node);
        let prev = self.table.insert(path.to_string(), node);
        assert!(prev.is_none());
        self.table.get(path)
    }

    pub fn remove(&mut self, path: &str) -> Option<Rc<NodeT>> {
        self.table.remove(path)
    }

    pub fn iter(&self) -> NodeTableIter<NodeT> {
        self.table.iter()
    }
}
