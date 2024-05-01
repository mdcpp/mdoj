use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    path::{Component, Path},
    sync::Arc,
};

use tokio::sync::RwLock;

fn to_internal_path<'a>(path: &'a impl AsRef<Path>) -> impl Iterator<Item = &'a OsStr> {
    path.as_ref()
        .components()
        .filter_map(|component| match component {
            Component::Prefix(x) => unreachable!("Windows only: {:?}", x),
            Component::RootDir | Component::CurDir | Component::ParentDir => None,
            Component::Normal(x) => Some(x),
        })
}

pub enum InsertResult<N> {
    AlreadyExists(N),
    Inserted(Option<N>),
    ParentNotFound,
    IsRoot,
}

pub struct LockNode<V: Sized> {
    children: HashMap<OsString, ArcRwNode<V>>,
    value: MaybeUninit<V>,
}

impl<V: Sized> Deref for LockNode<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        unsafe { self.value.assume_init_ref() }
    }
}

impl<V: Sized> DerefMut for LockNode<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.value.assume_init_mut() }
    }
}

type ArcRwNode<T> = Arc<RwLock<LockNode<T>>>;

impl<V: Sized> LockNode<V> {
    /// get child node by component
    #[inline]
    pub async fn get_by_component(&mut self, component: &OsStr) -> Option<ArcRwNode<V>> {
        self.children.get(component).cloned()
    }
    /// insert child node by component
    ///
    /// return the old child node if it exists
    pub async fn insert_component(
        &mut self,
        component: OsString,
        value: V,
    ) -> Option<ArcRwNode<V>> {
        self.children.insert(
            component,
            Arc::new(RwLock::new(LockNode {
                children: Default::default(),
                value: MaybeUninit::new(value),
            })),
        )
    }
    pub async fn remove_component(&mut self, component: &OsStr) -> Option<ArcRwNode<V>> {
        self.children.remove(component)
    }
    /// get child node by path
    pub async fn get_by_path(
        self_: ArcRwNode<V>,
        path: impl Iterator<Item = &OsStr>,
    ) -> Option<ArcRwNode<V>> {
        let mut root = self_;
        for component in path {
            let child = root.read().await.children.get(component)?.clone();
            root = child;
        }
        Some(root)
    }
    /// insert child node by path
    pub async fn insert_path(
        self_: ArcRwNode<V>,
        path: impl Iterator<Item = &OsStr>,
        value: V,
    ) -> InsertResult<ArcRwNode<V>> {
        let path = path.collect::<Vec<_>>();
        let mut root = self_;
        if path.is_empty() {
            return InsertResult::IsRoot;
        }
        let (last, path) = path.split_last().unwrap();
        for component in path {
            let child = match root.read().await.children.get(*component) {
                Some(child) => child.clone(),
                None => return InsertResult::ParentNotFound,
            };
            root = child;
        }
        let mut root = root.write().await;
        InsertResult::Inserted(root.insert_component(last.to_os_string(), value).await)
    }
}

impl<V: Sized> LockNode<V> {
    fn new(value: V) -> Self {
        Self {
            children: Default::default(),
            value: MaybeUninit::new(value),
        }
    }
    fn new_uninit() -> Self {
        Self {
            children: Default::default(),
            value: MaybeUninit::uninit(),
        }
    }
}

/// Path tree with partial locking
pub struct LockPathTree<V: Sized>(ArcRwNode<V>);

impl<V: Sized> LockPathTree<V> {
    pub fn new() -> Self {
        LockPathTree(Arc::new(RwLock::new(LockNode::new_uninit())))
    }
    /// insert path recursively
    #[inline]
    pub async fn insert_path(
        &self,
        path: impl AsRef<Path>,
        value: V,
    ) -> InsertResult<ArcRwNode<V>> {
        LockNode::insert_path(self.0.clone(), to_internal_path(&path), value).await
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
    mod lock {
        use super::*;

        #[tokio::test]
        async fn insert_lookup() {
            // tree.insert_path(Path::new("/abc/efg"), value)
        }
    }

    mod mutable {
        use super::*;
    }

    // #[tokio::test]
    // async fn lock
}
