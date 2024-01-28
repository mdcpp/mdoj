// pub trait Dump
// where
//     Self: Sized,
// {
//     fn serialize(self) -> Vec<u8>;
//     fn deserialize(raw: &[u8]) -> Result<(&[u8], Self), Error>;
// }

// impl Dump for () {
//     fn serialize(self) -> Vec<u8> {
//         Default::default()
//     }

//     fn deserialize(raw: &[u8]) -> Result<(&[u8], Self), Error> {
//         Ok((raw, ()))
//     }
// }

// impl Dump for i32 {
//     fn serialize(self) -> Vec<u8> {
//         let mut buffer = Vec::with_capacity(4);
//         let mut value = self as i64;
//         loop {
//             let mut tmp: i16 = (value & 0b0111_1111) as i16;
//             value >>= 7;
//             if value != 0 {
//                 tmp |= 0b1000_0000;
//             }
//             buffer.push((tmp as i8).to_be_bytes()[0]);
//             if value == 0 {
//                 break;
//             }
//         }
//         buffer
//     }

//     fn deserialize(raw: &[u8]) -> Result<(&[u8], Self), Error> {
//         let mut c = raw.into_iter();
//         let mut result: i32 = 0;
//         for num_read in 0..5 {
//             let read = *c.next().ok_or(Error::PaginationError("Not enough byte"))? as i32;
//             let value = read & 0b0111_1111;
//             result |= value << (7 * num_read);
//             if (read & 0b1000_0000) == 0 {
//                 break;
//             }
//         }
//         Ok((c.as_slice(), result))
//     }
// }
