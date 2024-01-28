#[macro_export]
macro_rules! assert_error {
    ($assert:expr,$msg:expr) => {
        if !$assert {
            return Err(Error::AssertFail($msg));
        }
    };
}

#[macro_export]
macro_rules! assert_eq_error {
    ($left:expr,$right:expr,$msg:expr) => {
        $crate::assert_error!($left == $right, $msg)
    };
}
