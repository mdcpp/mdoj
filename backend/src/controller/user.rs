use entity::{prelude::UserTable as User, user_table as user};
use openssl::sha;
use rand::{thread_rng, Rng};
use bincode;
use crate::controller::role;

pub struct UserData<'a>{
    name:&'a str,
    password:&'a str,
    description:&'a str
}

const SALT_LENGHT:usize=32;

fn hash(input:&[u8],salt:&[u8])->[u8;32]{
    let mut hasher = sha::Sha256::new();
    hasher.update(&[input,salt].concat());
    hasher.finish()
}

pub fn create_user(user:UserData){
    let mut rng=thread_rng();
    let salt:[u8;SALT_LENGHT]=rng.gen();

    let password_in_bytes=bincode::serialize(user.password).unwrap();

    let hash=hash(&password_in_bytes,&salt);

    todo!();
    // TODO
    // insert model into db
    // releationship
    
    
}