use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Component, Path},
};

const ID_MIN: usize = 1;
const MAX_ID_CAPACITY: u32 = 1 << 31;

/// convert a path to internal path(prefixes on the tree)
pub fn to_internal_path(path: &Path) -> impl Iterator<Item = &OsStr> {
    path.components().filter_map(|component| match component {
        Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
        Component::RootDir | Component::CurDir | Component::ParentDir => None,
        Component::Normal(x) => Some(x),
    })
}

/// A node on adjacency table
#[derive(Clone)]
struct Node<V> {
    parent_idx: usize,
    value: V,
    children: BTreeMap<OsString, usize>, // FIXME: use BtreeMap
}

/// A table to store the tree structure with
/// the ability to allocate id up to [`MAX_ID_CAPACITY`]
///
/// The table has ability to store a multiple disconnected tree
///
/// Note that cloning the table would actually clone the WHOLE tree
#[derive(Clone)]
pub struct AdjTable<V> {
    by_id: Vec<Node<V>>,
}

impl<V> AdjTable<V> {
    pub fn new() -> Self {
        Self { by_id: vec![] }
    }
    /// Insert a root node
    pub fn insert_root(&mut self, value: V) -> NodeWrapperMut<V> {
        let idx = self.by_id.len();
        self.by_id.push(Node {
            parent_idx: 0,
            value,
            children: BTreeMap::new(),
        });
        NodeWrapperMut { table: self, idx }
    }
    /// get first inserted node(one of the root node)
    ///
    /// # Panics
    /// It panic if the table is empty(there is no root node)
    pub fn get_first(&self) -> NodeWrapper<V> {
        NodeWrapper {
            table: self,
            idx: 0,
        }
    }
    /// get remain capacity of the table
    ///
    /// The capacity is the maximum number of ino that can be allocated
    pub fn get_remain_capacity(&self) -> u32 {
        MAX_ID_CAPACITY - self.by_id.len() as u32
    }
    /// get a node by id
    pub fn get(&self, id: usize) -> Option<NodeWrapper<V>> {
        if id < ID_MIN || id >= self.by_id.len() + ID_MIN {
            return None;
        }
        Some(NodeWrapper {
            table: self,
            idx: id - ID_MIN,
        })
    }
    /// get a mutable node by id
    pub fn get_mut(&mut self, id: usize) -> Option<NodeWrapperMut<V>> {
        if id < ID_MIN || id >= self.by_id.len() + ID_MIN {
            return None;
        }
        Some(NodeWrapperMut {
            table: self,
            idx: id - ID_MIN,
        })
    }
    /// get a node by path
    pub fn get_by_path<'a>(&self, path: impl Iterator<Item = &'a OsStr>) -> Option<NodeWrapper<V>> {
        let mut idx = self.get_first().idx;
        for name in path {
            if self.by_id[idx].children.contains_key(name) {
                idx = self.by_id[idx].children[name];
            } else {
                return None;
            }
        }
        Some(NodeWrapper { table: self, idx })
    }
    /// get a mutable node by path or inserted(if not exists)
    ///
    /// Note that it could create multiple nodes along the search
    pub fn get_by_path_or_insert<F>(
        &mut self,
        path: impl Iterator<Item = OsString>,
        mut default_value: F,
    ) -> NodeWrapperMut<V>
    where
        F: FnMut() -> V,
    {
        let mut idx: usize = self.get_first().idx;
        for name in path {
            if self.by_id[idx].children.contains_key(&name) {
                idx = self.by_id[idx].children[&name];
            } else {
                let new_idx = self.by_id.len();
                self.by_id.push(Node {
                    parent_idx: idx,
                    value: default_value(),
                    children: BTreeMap::new(),
                });
                // FIXME!
                idx = new_idx;
                self.by_id[idx].children.insert(name, new_idx);
            }
        }
        NodeWrapperMut { table: self, idx }
    }
    /// insert a node by path
    ///
    /// if the path is not exists, it will create the path
    /// (edge is filled with [`default_value()`])
    pub fn insert_by_path<'a, F>(
        &mut self,
        path: impl Iterator<Item = &'a OsStr>,
        mut default_value: F,
        value: V,
    ) -> NodeWrapperMut<V>
    where
        F: FnMut() -> V,
    {
        let mut idx = self.get_first().idx;
        let mut path = path.peekable();
        debug_assert!(path.peek().is_some());
        let mut seg;
        while path.peek().is_some() {
            seg = path.next().unwrap();
            if self.by_id[idx].children.contains_key(seg) {
                idx = self.by_id[idx].children[seg];
            } else {
                let new_idx = self.by_id.len();
                self.by_id.push(Node {
                    parent_idx: idx,
                    value: default_value(),
                    children: BTreeMap::new(),
                });
                self.by_id[idx].children.insert(seg.to_os_string(), new_idx);
                idx = new_idx;
            }
        }
        self.by_id[idx].value = value;
        NodeWrapperMut { table: self, idx }
    }
}

pub struct NodeWrapper<'a, V> {
    table: &'a AdjTable<V>,
    idx: usize,
}

impl<'a, V> NodeWrapper<'a, V> {
    /// get id of the node
    pub fn get_id(&self) -> usize {
        debug_assert!(self.idx < self.table.by_id.len());
        self.idx + ID_MIN
    }
    /// check if the node is root
    pub fn is_root(&self) -> bool {
        self.idx == 0
    }
    /// get parent node
    pub fn parent(&self) -> Option<NodeWrapper<'a, V>> {
        if self.is_root() {
            return None;
        }
        let parent_idx = self.table.by_id[self.idx].parent_idx;
        Some(NodeWrapper {
            table: self.table,
            idx: parent_idx,
        })
    }
    /// get children nodes' id
    pub fn children(self) -> impl Iterator<Item = usize> + 'a {
        self.table.by_id[self.idx]
            .children
            .iter()
            .map(|(_, &idx)| idx + ID_MIN)
    }
    /// get value of the node
    pub fn get_value(&self) -> &V {
        &self.table.by_id[self.idx].value
    }
    /// get name of the node
    pub fn get_name(&self) -> Option<&OsStr> {
        Some(if self.is_root() {
            OsStr::new("/")
        } else {
            self.table.by_id[self.table.by_id[self.idx].parent_idx]
                .children
                .iter()
                .find(|(_, &idx)| idx == self.idx)?
                .0
        })
    }
    /// get node by component
    pub fn get_by_component(&self, component: &OsStr) -> Option<NodeWrapper<V>> {
        if let Some(&idx) = self.table.by_id[self.idx].children.get(component) {
            Some(NodeWrapper {
                table: self.table,
                idx,
            })
        } else {
            None
        }
    }
}

pub struct NodeWrapperMut<'a, V> {
    table: &'a mut AdjTable<V>,
    idx: usize,
}

impl<'a, V> NodeWrapperMut<'a, V> {
    /// insert a node by component
    pub fn insert(&mut self, component: OsString, value: V) -> Option<NodeWrapperMut<V>> {
        if self.table.by_id[self.idx].children.contains_key(&component) {
            return None;
        }
        let idx = self.table.by_id.len();
        self.table.by_id.push(Node {
            parent_idx: self.idx,
            value,
            children: BTreeMap::new(),
        });
        self.table.by_id[self.idx].children.insert(component, idx);
        Some(NodeWrapperMut {
            table: self.table,
            idx,
        })
    }
    /// get id of the node
    pub fn get_id(&self) -> usize {
        NodeWrapper {
            table: self.table,
            idx: self.idx,
        }
        .get_id()
    }
    /// get value of the node
    pub fn get_value(&mut self) -> &mut V {
        &mut self.table.by_id[self.idx].value
    }
    /// get children node by component
    pub fn get_by_component(&mut self, component: &OsStr) -> Option<NodeWrapperMut<V>> {
        if let Some(&idx) = self.table.by_id[self.idx].children.get(component) {
            Some(NodeWrapperMut {
                table: self.table,
                idx,
            })
        } else {
            None
        }
    }
    /// remove children node by component
    ///
    /// note that it won't remove the node itself, only the edge
    pub fn remove_by_component(&mut self, component: &OsStr) -> bool {
        self.table.by_id[self.idx]
            .children
            .remove(component)
            .is_some()
    }
    /// get children nodes' id
    pub fn children(&mut self) -> impl Iterator<Item = usize> + '_ {
        self.table.by_id[self.idx]
            .children
            .iter()
            .map(|(_, &idx)| idx + ID_MIN)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_adj_table() {
        let mut table = super::AdjTable::new();
        let mut root = table.insert_root(0);
        root.insert(OsStr::new("a").into(), 1);
        let mut b = root.insert(OsStr::new("b").into(), 2).unwrap();

        let c = b.insert(OsStr::new("c").into(), 3).unwrap();

        assert_eq!(c.get_id(), 4);
        assert_eq!(b.children().collect::<Vec<_>>(), vec![4]);
    }
    #[test]
    fn get_or_insert() {
        let mut table = super::AdjTable::new();
        table.insert_root(0);
        table.insert_by_path(
            vec!["abc", "efg", "123", "456"]
                .into_iter()
                .map(|x| OsStr::new(x).into()),
            || 4,
            10,
        );
        let root = table.get_first();
        let l1 = root.children().next().unwrap();
        let l2 = table.get(l1).unwrap().children().next().unwrap();
        let l3 = table.get(l2).unwrap().children().next().unwrap();
        let l4 = table.get(l3).unwrap().children().next().unwrap();
        assert_eq!(l4, 5);
        assert_eq!(table.get(l4).unwrap().get_value(), &10);
    }
    #[test]
    fn parent_child_insert() {
        let mut table = super::AdjTable::new();
        let mut root = table.insert_root(0); // inode 1
        assert_eq!(root.get_id(), 1);
        let mut a = root.insert(OsStr::new("a").into(), 1).unwrap(); // inode 2
        assert_eq!(a.get_id(), 2);
        let c = a.insert(OsStr::new("c").into(), 3).unwrap(); // inode 3
        assert_eq!(c.get_id(), 3);
        let mut b = root.insert(OsStr::new("b").into(), 2).unwrap(); // inode 4
        assert_eq!(b.get_id(), 4);
        assert_eq!(b.get_value(), &2);
    }
}
