use std::{
    collections::HashMap,
    ffi::OsString,
    ops::{Deref, DerefMut},
    path::{Component, Path},
    sync::Arc,
};

use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

type ArcRwLock<T> = Arc<RwLock<T>>;

struct LockNode<V: Sized> {
    children: HashMap<OsString, ArcRwLock<LockNode<V>>>,
    value: Option<V>,
}

impl<V: Sized> Default for LockNode<V> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            value: Default::default(),
        }
    }
}

struct WriteNodeGuard<V: Sized>(OwnedRwLockWriteGuard<LockNode<V>>);

impl<V: Sized> Deref for WriteNodeGuard<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.0.value.as_ref().unwrap()
    }
}

impl<V: Sized> DerefMut for WriteNodeGuard<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.value.as_mut().unwrap()
    }
}

struct ReadNodeGuard<V: Sized>(OwnedRwLockReadGuard<LockNode<V>>);

impl<V: Sized> Deref for ReadNodeGuard<V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.0.value.as_ref().unwrap()
    }
}

/// Path tree with partial locking
pub struct LockPathTree<V: Sized>(ArcRwLock<LockNode<V>>);

impl<V: Sized> LockPathTree<V> {
    pub fn new() -> Self {
        LockPathTree(Arc::new(RwLock::new(LockNode::default())))
    }

    #[inline]
    async fn get_child(&self, path: impl AsRef<Path>) -> Option<ArcRwLock<LockNode<V>>> {
        let mut root = self.0.clone();
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
                Component::RootDir | Component::CurDir | Component::ParentDir => {
                    log::trace!("RootDir | CurDir | ParentDir");
                }
                Component::Normal(x) => {
                    let child = root.read().await.children.get(x)?.clone();
                    root = child;
                }
            }
        }
        Some(root)
    }
    pub async fn insert_path(&self, path: impl AsRef<Path>, value: V) -> Option<V> {
        let mut root = self.0.clone();
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
                Component::RootDir | Component::CurDir | Component::ParentDir => {
                    log::trace!("RootDir | CurDir | ParentDir");
                }
                Component::Normal(x) => {
                    let child = if let Some(child) = root.read().await.children.get(x) {
                        child.clone()
                    } else {
                        let node = Arc::new(RwLock::new(LockNode::default()));
                        root.write()
                            .await
                            .children
                            .insert(x.to_os_string(), node.clone());
                        node
                    };
                    root = child;
                }
            }
        }
        let mut root = root.write().await;
        root.value.replace(value)
    }
    pub async fn get_mut(
        &self,
        path: impl AsRef<Path>,
    ) -> Option<impl DerefMut + Deref<Target = V>> {
        match self.get_child(path).await {
            Some(root) => Some(WriteNodeGuard(root.write_owned().await)),
            None => None,
        }
    }
    pub async fn get(&self, path: impl AsRef<Path>) -> Option<impl Deref<Target = V> + Deref> {
        match self.get_child(path).await {
            Some(root) => Some(ReadNodeGuard(root.read_owned().await)),
            None => None,
        }
    }
}

struct Node<V: Sized> {
    children: HashMap<OsString, Node<V>>,
    value: Option<V>,
}

impl<V: Sized> Default for Node<V> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            value: Default::default(),
        }
    }
}

pub struct PathTree<V: Sized>(Node<V>);

impl<V: Sized> PathTree<V> {
    pub fn new() -> Self {
        PathTree(Node {
            children: HashMap::new(),
            value: None,
        })
    }
    #[inline]
    fn get_mut_child<'a>(&'a mut self, path: impl AsRef<Path>) -> Option<&'a mut Node<V>> {
        let mut root = &mut self.0;
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
                Component::RootDir | Component::CurDir | Component::ParentDir => {
                    log::trace!("RootDir | CurDir | ParentDir");
                }
                Component::Normal(x) => {
                    root = root.children.get_mut(x)?;
                }
            }
        }
        Some(root)
    }
    pub fn insert_path(&mut self, path: impl AsRef<Path>, value: V) -> Option<V> {
        let mut root = &mut self.0;
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
                Component::RootDir | Component::CurDir | Component::ParentDir => {
                    log::trace!("RootDir | CurDir | ParentDir");
                }
                Component::Normal(x) => {
                    // bypass borrow checker
                    match root.children.contains_key(x) {
                        true => {
                            root = root.children.get_mut(x).unwrap();
                        }
                        false => {
                            let node = Node {
                                children: HashMap::new(),
                                value: None,
                            };
                            root.children.insert(x.to_os_string(), node);
                            root = root.children.get_mut(x).unwrap();
                        }
                    };
                }
            }
        }
        root.value.replace(value)
    }
    #[inline]
    fn get_child(&self, path: impl AsRef<Path>) -> Option<&Node<V>> {
        let mut root = &self.0;
        for component in path.as_ref().components() {
            match component {
                Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
                Component::RootDir | Component::CurDir | Component::ParentDir => {
                    log::trace!("RootDir | CurDir | ParentDir");
                }
                Component::Normal(x) => {
                    root = root.children.get(x)?;
                }
            }
        }
        Some(root)
    }
    fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut V> {
        self.get_mut_child(path)
            .and_then(|node| node.value.as_mut())
    }
    fn get(&self, path: impl AsRef<Path>) -> Option<&V> {
        self.get_child(path).and_then(|node| node.value.as_ref())
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
