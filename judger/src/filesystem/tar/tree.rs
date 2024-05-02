use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::{Component, Path},
};

fn to_internal_path<'a>(path: &'a impl AsRef<Path>) -> impl Iterator<Item = &'a OsStr> {
    path.as_ref()
        .components()
        .filter_map(|component| match component {
            Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
            Component::RootDir | Component::CurDir | Component::ParentDir => None,
            Component::Normal(x) => Some(x),
        })
}

struct Node<V: Sized> {
    children: HashMap<OsString, Node<V>>,
    value: V,
}

impl<V: Sized> Node<V> {
    fn new(value: V) -> Self {
        Self {
            children: Default::default(),
            value,
        }
    }
}

pub struct Tree<V: Sized>(Node<V>);

impl<V: Sized> Tree<V>
where
    V: Default,
{
    pub fn new() -> Self {
        Tree(Node::new(Default::default()))
    }
    pub fn insert_path(&mut self, path: impl AsRef<Path>, mut value: V) -> Option<V>
    where
        V: Default,
    {
        let mut root = &mut self.0;
        let path = to_internal_path(&path).collect::<Vec<_>>();
        let (last, path) = path.split_last().unwrap();
        for component in path {
            root = root
                .children
                .entry(component.to_os_string())
                .or_insert_with(|| Node::new(Default::default()));
        }
        match root.children.get_mut(*last) {
            Some(x) => {
                std::mem::swap(&mut x.value, &mut value);
                Some(value)
            }
            None => None,
        }
    }
    pub fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut V> {
        let this = &mut *self;
        let mut root = &mut this.0;
        for component in to_internal_path(&path) {
            root = root.children.get_mut(component)?;
        }
        Some(&mut root.value)
    }
    pub fn get(&self, path: impl AsRef<Path>) -> Option<&V> {
        let this = &self;
        let mut root = &this.0;
        for component in to_internal_path(&path) {
            root = root.children.get(component)?;
        }
        Some(&root.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
