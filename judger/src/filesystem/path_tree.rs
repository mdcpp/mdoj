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

#[derive(Debug)]
pub enum InsertResult<N> {
    AlreadyExists(N),
    Inserted(Option<N>),
    ParentNotFound,
    IsRoot,
}

impl<N> PartialEq for InsertResult<Arc<N>> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AlreadyExists(l0), Self::AlreadyExists(r0)) => Arc::ptr_eq(l0, r0),
            (Self::Inserted(l0), Self::Inserted(r0)) => match (l0, r0) {
                (Some(l0), Some(r0)) => Arc::ptr_eq(l0, r0),
                (None, None) => true,
                _ => false,
            },
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Debug)]
pub struct LockNode<V: Sized> {
    children: HashMap<OsString, ArcRwNode<V>>,
    value: V,
}

impl<V: Sized> Deref for LockNode<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V: Sized> DerefMut for LockNode<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

type ArcRwNode<T> = Arc<RwLock<LockNode<T>>>;

impl<V: Sized> LockNode<V> {
    fn new(value: V) -> Self {
        Self {
            children: Default::default(),
            value,
        }
    }
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
        self.children
            .insert(component, Arc::new(RwLock::new(LockNode::new(value))))
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
        let path = path.peekable();
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

/// Path tree with partial locking
#[derive(Clone)]
pub struct LockPathTree<V: Sized>(ArcRwNode<V>);

impl<V: Sized> LockPathTree<V> {
    pub fn new(root: V) -> Self {
        LockPathTree(Arc::new(RwLock::new(LockNode::new(root))))
    }
    pub async fn get_by_path(&self, path: impl AsRef<Path>) -> Option<ArcRwNode<V>> {
        LockNode::get_by_path(self.0.clone(), to_internal_path(&path)).await
    }
    /// insert path
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

pub struct PathTree<V: Sized>(Node<V>);

impl<V: Sized> PathTree<V>
where
    V: Default,
{
    pub fn new() -> Self {
        PathTree(Node::new(Default::default()))
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
    fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut V> {
        let this = &mut *self;
        let mut root = &mut this.0;
        for component in to_internal_path(&path) {
            root = root.children.get_mut(component)?;
        }
        Some(&mut root.value)
    }
    fn get(&self, path: impl AsRef<Path>) -> Option<&V> {
        let this = &self;
        let mut root = &this.0;
        for component in to_internal_path(&path) {
            root = root.children.get(component)?;
        }
        Some(&root.value)
    }
}

#[cfg(test)]
mod lock_test {
    use super::*;

    #[tokio::test]
    async fn insert_parent_not_found() {
        let tree = LockPathTree::new(0);
        assert_eq!(
            tree.insert_path("a/b/c", 1).await,
            InsertResult::ParentNotFound
        );
    }
    #[tokio::test]
    async fn insert_is_root() {
        let tree = LockPathTree::new(0);
        assert_eq!(tree.insert_path("", 1).await, InsertResult::IsRoot);
    }
    #[tokio::test]
    async fn insert() {
        let tree = LockPathTree::new(0);
        macro_rules! insert {
            ($path:expr, $val:expr) => {
                assert_eq!(
                    tree.insert_path($path, $val).await,
                    InsertResult::Inserted(None)
                );
            };
        }
        macro_rules! lookup {
            ($path:expr,$val:expr) => {
                assert_eq!(
                    tree.get_by_path($path).await.unwrap().read().await.value,
                    $val
                );
            };
        }

        insert!("a", 1);
        insert!("a/u", 2);
        insert!("a/h", 3);
        insert!("a/h/f", 4);
        lookup!("/", 0);
        lookup!("a", 1);
        lookup!("a/u", 2);
        lookup!("a/h", 3);
        lookup!("a/h/f", 4);
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn multi_lookup() {
        let tree = LockPathTree::new(0);
        tree.insert_path("a", 1).await;
        tree.insert_path("a/u", 2).await;
        tree.insert_path("a/h", 3).await;
        tree.insert_path("a/h/f", 4).await;
        async fn lookup(tree: &LockPathTree<i32>, path: &str, val: i32) {
            for _ in 0..30 {
                let tree = tree.clone();
                let path = path.to_string();
                tokio::spawn(async move {
                    for _ in 0..300 {
                        assert_eq!(
                            tree.get_by_path(&path).await.unwrap().read().await.value,
                            val
                        );
                    }
                })
                .await
                .unwrap();
            }
        }
        tokio::join!(
            lookup(&tree, "a", 1),
            lookup(&tree, "a/u", 2),
            lookup(&tree, "a/h", 3),
            lookup(&tree, "a/h/f", 4)
        );
    }
    #[cfg(taregt_os = "windows")]
    #[tokio::test]
    #[should_panic]
    async fn windows() {
        let tree = LockPathTree::new(0);
        tree.insert_path("C:\\a", 1).await;
    }
}
#[cfg(test)]
mod test {
    use super::*;
}
