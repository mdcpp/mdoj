// macro_rules! set_bit_value {
//     ($item:ident,$name:ident,$pos:expr) => {
//         paste::paste! {
//             impl $item{
//                 pub fn [<can_ $name>](&self)->bool{
//                     let filter = 1_i64<<($pos);
//                     (self.0&filter) == filter
//                 }
//                 pub fn [<grant_ $name>](&mut self,value:bool){
//                     let filter = 1_i64<<($pos);
//                     if (self.0&filter == filter) ^ value{
//                         self.0 = self.0 ^ filter;
//                     }
//                 }
//             }
//         }
//     };
// }

// #[derive(Debug, Clone, Copy)]
// pub struct UserPermBytes(pub i64);

// impl UserPermBytes {
//     pub fn strict_ge(&self, other: Self) -> bool {
//         (self.0 | other.0) == other.0
//     }
// }

// set_bit_value!(UserPermBytes, root, 0);
// set_bit_value!(UserPermBytes, manage_problem, 1);
// set_bit_value!(UserPermBytes, manage_education, 2);
// set_bit_value!(UserPermBytes, manage_announcement, 3);
// set_bit_value!(UserPermBytes, manage_submit, 4);
// set_bit_value!(UserPermBytes, publish, 5);
// set_bit_value!(UserPermBytes, link, 6);

// // #[cfg(test)]
// // mod test {
// //     #[test]
// //     fn test_pos_bool() {
// //         struct TestFlag<'a>(&'a mut i64);
// //         set_bit_value!(TestFlag, attr_c, 1);
// //         let mut a = 0;
// //         let mut perm = TestFlag(&mut a);
// //         perm.set_attr_c(true);
// //         perm.set_attr_c(true);
// //         assert!(perm.get_attr_c());
// //         perm.set_attr_c(false);
// //         perm.set_attr_c(false);
// //         assert!(!perm.get_attr_c());
// //     }
// // }
