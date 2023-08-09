use rand::Rng;


pub struct TokenController{
}

impl TokenController {
    pub async fn add(&self,user_id:i32,token:Vec<u8>){
        let mut rng = rand::thread_rng();
        let rand: i128 = rng.gen();
        let rand=rand.to_be_bytes();
        // entity::token::ActiveModel{ id: todo!(), user_id: todo!(), rand: todo!() }
        todo!();
    }
}