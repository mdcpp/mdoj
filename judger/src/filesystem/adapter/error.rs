#[derive(thiserror::Error, Debug)]
pub enum FuseError {
    #[error("not a readable file")]
    IsDir,
    #[error("end of file")]
    Eof,
    #[error("not a dir")]
    NotDir,
    #[error("out of resource")]
    OutOfPermit,
    #[error("number too large")]
    OutOfRange,
    #[error("unimplemented")]
    Unimplemented,
    #[error("missed inode")]
    InvaildIno,
    #[error("missed handle")]
    HandleNotFound,
    #[error("underlaying file error")]
    Underlaying,
    #[error("invalid path")]
    InvalidPath,
}

impl From<FuseError> for fuse3::Errno {
    fn from(value: FuseError) -> Self {
        match value {
            FuseError::IsDir => libc::EISDIR,
            FuseError::NotDir => libc::ENOTDIR,
            FuseError::Eof => libc::EOF,
            FuseError::OutOfPermit => {
                log::info!("out of resource");
                libc::ENOMEM
            }
            FuseError::InvalidPath | FuseError::InvaildIno => libc::ENOENT,
            _ => {
                log::warn!("FUSE driver broken: {}", value);
                libc::ENOMEM
            }
        }
        .into()
    }
}
