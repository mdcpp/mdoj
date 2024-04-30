mod fuse;
mod macro_;
mod overlay;
mod path_tree;
mod table;
mod tar;

type INODE = u64;
type HANDLE = u64;

use path_tree::*;
