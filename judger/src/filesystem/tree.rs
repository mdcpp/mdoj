use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
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
pub struct Node<V: Sized> {
    children: HashMap<OsString, ArcNode<V>>,
    value: V,
}

impl<V: Sized> Deref for Node<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V: Sized> DerefMut for Node<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub type ArcNode<T> = Arc<RwLock<Node<T>>>;

impl<V: Sized> Node<V> {
    fn new(value: V) -> Self {
        Self {
            children: Default::default(),
            value,
        }
    }
    /// get child node by component
    #[inline]
    pub fn get_by_component(&self, component: &OsStr) -> Option<ArcNode<V>> {
        self.children.get(component).cloned()
    }
    /// insert child node by component
    ///
    /// return the old child node if it exists
    pub fn insert_component(&mut self, component: OsString, value: V) -> Option<ArcNode<V>> {
        self.children
            .insert(component, Arc::new(RwLock::new(Node::new(value))))
    }
    pub fn remove_component(&mut self, component: &OsStr) -> Option<ArcNode<V>> {
        self.children.remove(component)
    }
    /// get child node by path
    pub async fn get_by_path(
        self_: ArcNode<V>,
        path: impl Iterator<Item = &OsStr>,
    ) -> Option<ArcNode<V>> {
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
        self_: ArcNode<V>,
        path: impl Iterator<Item = &OsStr>,
        value: V,
    ) -> InsertResult<ArcNode<V>> {
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
        InsertResult::Inserted(root.insert_component(last.to_os_string(), value))
    }
}

/// Path tree with partial locking
#[derive(Clone)]
pub struct Tree<V: Sized>(ArcNode<V>);

impl<V: Sized> Tree<V> {
    pub fn new(root: V) -> Self {
        Tree(Arc::new(RwLock::new(Node::new(root))))
    }
    pub async fn get_by_path(&self, path: impl AsRef<Path>) -> Option<ArcNode<V>> {
        Node::get_by_path(self.0.clone(), to_internal_path(&path)).await
    }
    /// insert path
    #[inline]
    pub async fn insert_path(&self, path: impl AsRef<Path>, value: V) -> InsertResult<ArcNode<V>> {
        Node::insert_path(self.0.clone(), to_internal_path(&path), value).await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn insert_parent_not_found() {
        let tree = Tree::new(0);
        assert_eq!(
            tree.insert_path("a/b/c", 1).await,
            InsertResult::ParentNotFound
        );
    }
    #[tokio::test]
    async fn insert_is_root() {
        let tree = Tree::new(0);
        assert_eq!(tree.insert_path("", 1).await, InsertResult::IsRoot);
    }
    #[tokio::test]
    async fn insert() {
        let tree = Tree::new(0);
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
        let tree = Tree::new(0);
        tree.insert_path("a", 1).await;
        tree.insert_path("a/u", 2).await;
        tree.insert_path("a/h", 3).await;
        tree.insert_path("a/h/f", 4).await;
        async fn lookup(tree: &Tree<i32>, path: &str, val: i32) {
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
        let tree = Tree::new(0);
        tree.insert_path("C:\\a", 1).await;
    }
}
