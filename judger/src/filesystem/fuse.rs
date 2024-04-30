// use fuse3::{raw::{reply::*, Request}, Errno};
// use futures_core::Future;
// use tokio::fs::File;

// use super::table::{HandleTable, INodeTable};

// pub struct FileSystem {
//     inode_table: INodeTable<File>,
//     handle_table: HandleTable<File>,
// }

// impl fuse3::raw::Filesystem for FileSystem {
//     fn init(&self, req: Request) -> impl Future<Output = Result<ReplyInit,Errno>> + Send {
//         todo!()
//     }

//     fn destroy(&self, req: Request) -> impl Future<Output = ()> + Send {
//         todo!()
//     }

//     #[doc = r" dir entry stream given by [`readdir`][Filesystem::readdir]."]
//     type DirEntryStream<'a>
//     where
//         Self: 'a;

//     #[doc = r" dir entry plus stream given by [`readdirplus`][Filesystem::readdirplus]."]
//     type DirEntryPlusStream<'a>
//     where
//         Self: 'a;
// }
