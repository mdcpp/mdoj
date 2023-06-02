macro_rules! set_bit_value {
    ($item:ident,$name:ident,$pos:expr) => {
        paste::paste! {
            impl $item{
                pub fn [<get_ $name>](&self)->bool{
                    let filter = 1_i64<<($pos);
                    self.0&filter == filter
                }
                pub fn [<set_ $name>](&mut self,value:bool){
                    let filter = 1_i64<<($pos);
                    if (self.0&filter == filter) ^ value{
                        self.0 = self.0 ^ filter;
                    }
                }
            }
        }
    };
}

#[derive(Default)]
pub struct UserPermBytes(i64);

impl UserPermBytes {
    pub fn strict_ge(&self, other: Self) -> bool {
        (self.0 | other.0) == other.0
    }
}

set_bit_value!(UserPermBytes, create_group, 0);
set_bit_value!(UserPermBytes, delete_group, 1);
set_bit_value!(UserPermBytes, create_user, 2);
set_bit_value!(UserPermBytes, root, 3);

#[derive(Default)]
pub struct GroupPermBytes(i64);

impl GroupPermBytes {
    pub fn strict_ge(&self, other: Self) -> bool {
        (self.0 | other.0) == other.0
    }
}

set_bit_value!(GroupPermBytes, create_problem, 0);
set_bit_value!(GroupPermBytes, edit_problem, 1);
set_bit_value!(GroupPermBytes, delete_problem, 2);
set_bit_value!(GroupPermBytes, create_edu, 3);
set_bit_value!(GroupPermBytes, edit_edu, 4);
set_bit_value!(GroupPermBytes, delete_edu, 5);
set_bit_value!(GroupPermBytes, add_user, 6);

#[cfg(test)]
mod test {
    #[test]
    fn test_pos_bool() {
        struct TestFlag(i64);
        set_bit_value!(TestFlag, attr_c, 1);
        let mut perm = TestFlag(0);
        perm.set_attr_c(true);
        perm.set_attr_c(true);
        assert!(perm.get_attr_c());
        perm.set_attr_c(false);
        perm.set_attr_c(false);
        assert!(!perm.get_attr_c());
    }
}
