use fuse3::FileType;

mod adapter;
mod macro_;
mod overlay;
mod table;
mod tar;
mod tree;

trait EntryTrait {
    fn kind(&self) -> FileType;
}
