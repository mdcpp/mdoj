use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    ops::{Deref, DerefMut},
    path::{Component, Path},
    sync::Arc,
};

use futures_core::{future::BoxFuture, Future};
use tokio::sync::RwLock;

pub fn arc_lock<T>(x: T) -> Arc<RwLock<T>> {
    Arc::new(RwLock::new(x))
}

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
    pub fn new(value: V) -> ArcNode<V> {
        Arc::new(RwLock::new(Self {
            children: Default::default(),
            value,
        }))
    }
    fn arc_clone<'a>(x: &'a mut ArcNode<V>) -> BoxFuture<'a, ()>
    where
        V: Clone + Send + Sync + 'static,
    {
        Box::pin(async move {
            let mut node = {
                let lock = x.deref().read().await;
                let value = lock.deref().value.clone();
                let children = lock.deref().children.clone();
                Node { value, children }
            };
            for (_, mut v) in node.children.iter_mut() {
                Self::arc_clone(&mut v).await;
            }
            *x = arc_lock(node);
        })
    }
    /// get child node by component
    #[inline]
    pub fn get_by_component(&self, component: &OsStr) -> Option<ArcNode<V>> {
        self.children.get(component).cloned()
    }
    /// insert child node by component
    ///
    /// return the old child node if it exists
    pub fn insert_component(
        &mut self,
        component: OsString,
        value: ArcNode<V>,
    ) -> Option<ArcNode<V>> {
        self.children.insert(component, value)
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
    async fn insert_closure<F>(
        self_: ArcNode<V>,
        path: impl Iterator<Item = &OsStr>,
        value: ArcNode<V>,
        mut f: F,
    ) -> InsertResult<ArcNode<V>>
    where
        F: FnMut(&OsStr, &mut Node<V>) -> Result<ArcNode<V>, InsertResult<ArcNode<V>>>,
    {
        let path = path.collect::<Vec<_>>();
        let mut root = self_;
        if path.is_empty() {
            return InsertResult::IsRoot;
        }
        let (last, path) = path.split_last().unwrap();
        for component in path {
            let child = match f(component, root.write().await.deref_mut()) {
                Ok(x) => x,
                Err(x) => {
                    return x;
                }
            };
            root = child;
        }
        let mut root = root.write().await;
        InsertResult::Inserted(root.insert_component(last.to_os_string(), value))
    }
}

/// Path tree with partial locking
pub struct Tree<V: Sized>(ArcNode<V>);

impl<V: Sized> Default for Tree<V>
where
    V: Default,
{
    fn default() -> Self {
        Self(Node::new(Default::default()))
    }
}

impl<V: Sized> Tree<V> {
    pub fn new(root: ArcNode<V>) -> Self {
        Tree(root)
    }
    pub fn cloned(&self) -> Self {
        Self(self.0.clone())
    }
    pub async fn clone(&self) -> Self
    where
        V: Clone + Send + Sync + 'static,
    {
        let mut new_node = self.0.clone();
        Node::arc_clone(&mut new_node).await;
        Self(new_node)
    }
    pub async fn get_by_path(&self, path: impl AsRef<Path>) -> Option<ArcNode<V>> {
        Node::get_by_path(self.0.clone(), to_internal_path(&path)).await
    }
    pub async fn insert_path(
        &self,
        path: impl AsRef<Path>,
        value: ArcNode<V>,
    ) -> InsertResult<ArcNode<V>> {
        let path = to_internal_path(&path);
        Node::insert_closure(self.0.clone(), path, value, |component, root| {
            match root.children.get(component) {
                Some(child) => Ok(child.clone()),
                None => return Err(InsertResult::ParentNotFound),
            }
        })
        .await
    }
    pub fn get_root(&self) -> ArcNode<V> {
        self.0.clone()
    }
    pub async fn insert_path_recursive<F>(
        &self,
        path: impl AsRef<Path>,
        value: ArcNode<V>,
        mut default: F,
    ) -> InsertResult<ArcNode<V>>
    where
        F: FnMut() -> ArcNode<V>,
    {
        let path = to_internal_path(&path);
        Node::insert_closure(self.0.clone(), path, value, |component, root| {
            Ok(root
                .children
                .entry(component.to_os_string())
                .or_insert_with(|| default())
                .clone())
        })
        .await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn insert_parent_not_found() {
        let tree = Tree::new(Node::new(0));
        assert_eq!(
            tree.insert_path("a/b/c", Node::new(1)).await,
            InsertResult::ParentNotFound
        );
    }
    #[tokio::test]
    async fn insert_is_root() {
        let tree = Tree::new(Node::new(0));
        assert_eq!(
            tree.insert_path("", Node::new(1)).await,
            InsertResult::IsRoot
        );
    }
    #[tokio::test]
    async fn insert() {
        let tree = Tree::new(Node::new(0));
        macro_rules! insert {
            ($path:expr, $val:expr) => {
                assert_eq!(
                    tree.insert_path($path, Node::new($val)).await,
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
        let tree = Tree::new(Node::new(0));
        tree.insert_path("a", Node::new(1)).await;
        tree.insert_path("a/u", Node::new(2)).await;
        tree.insert_path("a/h", Node::new(3)).await;
        tree.insert_path("a/h/f", Node::new(4)).await;
        async fn lookup(tree: &Tree<i32>, path: &str, val: i32) {
            for _ in 0..30 {
                let tree = tree.cloned();
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
    #[tokio::test]
    async fn deep_clone() {
        let tree_a = Tree::new(Node::new(0));
        let tree_b = tree_a.clone().await;
        tree_a.insert_path("a", Node::new(1)).await;
        tree_a.insert_path("a/u", Node::new(2)).await;
        tree_a.insert_path("a/h", Node::new(3)).await;
        tree_a.insert_path("a/h/f", Node::new(4)).await;
        assert!(tree_a.get_by_path("a/h/f").await.is_some());
        assert!(tree_b.get_by_path("a/h/f").await.is_none());
        assert!(tree_b.get_by_path("a").await.is_none());
    }
    #[cfg(taregt_os = "windows")]
    #[tokio::test]
    #[should_panic]
    async fn windows() {
        let tree = Tree::new(Node::new(0));
        tree.insert_path("C:\\a", 1).await;
    }
}
