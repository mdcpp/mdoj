use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
};

const ID_MIN: usize = 1;

struct Node<V> {
    parent_idx: usize,
    value: V,
    children: HashMap<OsString, usize>,
}

pub struct AdjTable<V> {
    by_id: Vec<Node<V>>,
}

impl<V> AdjTable<V> {
    pub fn new() -> Self {
        Self { by_id: vec![] }
    }
    pub fn insert_root(&mut self, value: V) -> NodeWrapper<V> {
        let idx = self.by_id.len();
        self.by_id.push(Node {
            parent_idx: 0,
            value,
            children: HashMap::new(),
        });
        NodeWrapper { table: self, idx }
    }
    pub fn get_root(&mut self) -> NodeWrapper<V> {
        NodeWrapper {
            table: self,
            idx: 0,
        }
    }
    pub fn get_by_id(&mut self, id: usize) -> Option<NodeWrapper<V>> {
        if id < ID_MIN || id >= self.by_id.len() + ID_MIN {
            return None;
        }
        Some(NodeWrapper {
            table: self,
            idx: id - ID_MIN,
        })
    }
    pub fn get_by_path<'a>(
        &mut self,
        mut path: impl Iterator<Item = &'a OsStr>,
    ) -> Option<NodeWrapper<V>> {
        let mut idx = self.get_root().idx;
        while let Some(name) = path.next() {
            if self.by_id[idx].children.contains_key(name) {
                idx = self.by_id[idx].children[name];
            } else {
                return None;
            }
        }
        Some(NodeWrapper { table: self, idx })
    }
    pub fn get_by_path_or_insert<F>(
        &mut self,
        path: impl Iterator<Item = OsString>,
        mut default_value: F,
    ) -> NodeWrapper<V>
    where
        F: FnMut() -> V,
    {
        let mut idx = self.get_root().idx;
        for name in path {
            if self.by_id[idx].children.contains_key(&name) {
                idx = self.by_id[idx].children[&name];
            } else {
                let new_idx = self.by_id.len();
                self.by_id.push(Node {
                    parent_idx: idx,
                    value: default_value(),
                    children: HashMap::new(),
                });
                self.by_id[idx].children.insert(name, new_idx);
            }
        }
        NodeWrapper { table: self, idx }
    }
    pub fn insert_by_path<F>(
        &mut self,
        path: impl Iterator<Item = OsString>,
        mut default_value: F,
        value: V,
    ) -> NodeWrapper<V>
    where
        F: FnMut() -> V,
    {
        let mut idx = self.get_root().idx;
        let mut path = path.peekable();
        debug_assert!(path.peek().is_some());
        let mut seg;
        while path.peek().is_some() {
            seg = path.next().unwrap();
            if self.by_id[idx].children.contains_key(&seg) {
                idx = self.by_id[idx].children[&seg];
            } else {
                let new_idx = self.by_id.len();
                self.by_id.push(Node {
                    parent_idx: idx,
                    value: default_value(),
                    children: HashMap::new(),
                });
                self.by_id[idx].children.insert(seg, new_idx);
                idx = new_idx;
            }
        }
        self.by_id[idx].value = value;
        NodeWrapper { table: self, idx }
    }
}

pub struct NodeWrapper<'a, V> {
    table: &'a mut AdjTable<V>,
    idx: usize,
}

impl<'a, V> NodeWrapper<'a, V> {
    pub fn get_id(&self) -> usize {
        self.idx + ID_MIN
    }
    pub fn is_root(&self) -> bool {
        self.idx == 0
    }
    pub fn insert(&mut self, name: OsString, value: V) -> NodeWrapper<V> {
        let idx = self.table.by_id.len();
        self.table.by_id.push(Node {
            parent_idx: self.idx,
            value,
            children: HashMap::new(),
        });
        self.table.by_id[self.idx].children.insert(name, idx);
        NodeWrapper {
            table: self.table,
            idx,
        }
    }
    pub fn parent(self) -> Option<NodeWrapper<'a, V>> {
        if self.idx == 0 {
            return None;
        }
        let parent_idx = self.table.by_id[self.idx].parent_idx;
        Some(NodeWrapper {
            table: self.table,
            idx: parent_idx,
        })
    }
    fn children(self) -> impl Iterator<Item = usize> + 'a {
        self.table.by_id[self.idx]
            .children
            .iter()
            .map(|(_, &idx)| idx + ID_MIN)
    }
    fn get_value(&self) -> &V {
        &self.table.by_id[self.idx].value
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
        let mut b = root.insert(OsStr::new("b").into(), 2);

        let c = b.insert(OsStr::new("c").into(), 3);

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
        let root = table.get_root();
        let l1 = root.children().next().unwrap();
        let l2 = table.get_by_id(l1).unwrap().children().next().unwrap();
        let l3 = table.get_by_id(l2).unwrap().children().next().unwrap();
        let l4 = table.get_by_id(l3).unwrap().children().next().unwrap();
        assert_eq!(l4, 5);
        assert_eq!(table.get_by_id(l4).unwrap().get_value(), &10);
    }
}
