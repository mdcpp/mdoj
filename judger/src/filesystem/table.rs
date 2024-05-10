use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    path::{Component, Path},
};

const ID_MIN: usize = 1;
const REMAIN_CAPACITY: u32 = 1 << 31;

pub fn to_internal_path<'a>(path: &'a Path) -> impl Iterator<Item = &OsStr> + 'a {
    path.components().filter_map(|component| match component {
        Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
        Component::RootDir | Component::CurDir | Component::ParentDir => None,
        Component::Normal(x) => Some(x),
    })
    // .collect::<Vec<_>>()
}

pub trait DeepClone {
    async fn deep_clone(&self) -> Self;
}

#[derive(Clone)]
struct Node<V> {
    parent_idx: usize,
    value: V,
    children: BTreeMap<OsString, usize>, // FIXME: use BtreeMap
}

impl<V: DeepClone> DeepClone for Node<V> {
    async fn deep_clone(&self) -> Self {
        Self {
            parent_idx: self.parent_idx,
            value: self.value.deep_clone().await,
            children: self.children.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        }
    }
}

#[derive(Clone)]
pub struct AdjTable<V> {
    by_id: Vec<Node<V>>,
}

impl<V> AdjTable<V> {
    pub fn new() -> Self {
        Self { by_id: vec![] }
    }
    pub fn insert_root(&mut self, value: V) -> NodeWrapperMut<V> {
        let idx = self.by_id.len();
        self.by_id.push(Node {
            parent_idx: 0,
            value,
            children: BTreeMap::new(),
        });
        NodeWrapperMut { table: self, idx }
    }
    pub fn get_root(&self) -> NodeWrapper<V> {
        NodeWrapper {
            table: self,
            idx: 0,
        }
    }
    pub fn get_remain_capacity(&self) -> u32 {
        REMAIN_CAPACITY - self.by_id.len() as u32
    }
    pub fn get(&self, id: usize) -> Option<NodeWrapper<V>> {
        if id < ID_MIN || id >= self.by_id.len() + ID_MIN {
            return None;
        }
        Some(NodeWrapper {
            table: self,
            idx: id - ID_MIN,
        })
    }
    pub fn get_mut(&mut self, id: usize) -> Option<NodeWrapperMut<V>> {
        if id < ID_MIN || id >= self.by_id.len() + ID_MIN {
            return None;
        }
        Some(NodeWrapperMut {
            table: self,
            idx: id - ID_MIN,
        })
    }
    pub fn get_by_path<'a>(
        &self,
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
    ) -> NodeWrapperMut<V>
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
                    children: BTreeMap::new(),
                });
                self.by_id[idx].children.insert(name, new_idx);
            }
        }
        NodeWrapperMut { table: self, idx }
    }
    pub fn insert_by_path<'a, F>(
        &mut self,
        path: impl Iterator<Item = &'a OsStr>,
        mut default_value: F,
        value: V,
    ) -> NodeWrapperMut<V>
    where
        F: FnMut() -> V,
    {
        let mut idx = self.get_root().idx;
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
    pub fn get_id(&self) -> usize {
        self.idx + ID_MIN
    }
    pub fn is_root(&self) -> bool {
        self.idx == 0
    }
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
    pub fn children(self) -> impl Iterator<Item = usize> + 'a {
        self.table.by_id[self.idx]
            .children
            .iter()
            .map(|(_, &idx)| idx + ID_MIN)
    }
    pub fn get_value(&self) -> &V {
        &self.table.by_id[self.idx].value
    }
    pub fn get_name(&self) -> &OsStr {
        if self.is_root() {
            OsStr::new("/")
        } else {
            self.table.by_id[self.table.by_id[self.idx].parent_idx]
                .children
                .iter()
                .find(|(_, &idx)| idx == self.idx)
                .unwrap()
                .0
        }
    }
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
    pub fn insert(&mut self, name: OsString, value: V) -> NodeWrapperMut<V> {
        let idx = self.table.by_id.len();
        self.table.by_id.push(Node {
            parent_idx: self.idx,
            value,
            children: BTreeMap::new(),
        });
        self.table.by_id[self.idx].children.insert(name, idx);
        NodeWrapperMut {
            table: self.table,
            idx,
        }
    }
    pub fn get_id(&self) -> usize {
        NodeWrapper {
            table: self.table,
            idx: self.idx,
        }
        .get_id()
    }
    pub fn get_value(&mut self) -> &mut V {
        &mut self.table.by_id[self.idx].value
    }
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
        let l2 = table.get(l1).unwrap().children().next().unwrap();
        let l3 = table.get(l2).unwrap().children().next().unwrap();
        let l4 = table.get(l3).unwrap().children().next().unwrap();
        assert_eq!(l4, 5);
        assert_eq!(table.get(l4).unwrap().get_value(), &10);
    }
}
