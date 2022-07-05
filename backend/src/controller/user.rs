use entity::{prelude::UserTable as User, user_table as user};
use openssl::sha;
use rand::{thread_rng, Rng};
use bincode;
use sea_orm::{EntityTrait, Set, ActiveModelTrait, DatabaseConnection};
use crate::controller::role;

pub struct Userstate<'a>{
    conn: &'a DatabaseConnection,
}

impl<'a> Userstate<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Userstate<'a> {
        Userstate {
            conn: &conn,
        }
    }
}

pub struct UserData<'a>{
    name:&'a str,
    password:&'a str
}

const SALT_LENGTH:usize=32;

fn hash(input:&[u8],salt:&[u8])->[u8;32]{
    let mut hasher = sha::Sha256::new();
    hasher.update(&[input,salt].concat());
    hasher.finish()
}

pub async fn create_user<'a>(state:&Userstate<'a>,user:UserData<'a>){
    let mut rng=thread_rng();
    let salt:[u8;SALT_LENGTH]=rng.gen();

    let password_in_bytes=bincode::serialize(user.password).unwrap();

    let hash=hash(&password_in_bytes,&salt);

    // TODO
    // releationship: role
    
    (user::ActiveModel{
        name_user:Set(user.name.to_string()),
        hashed_password:Set(hash.to_vec()),
        salt:Set(salt.to_vec()),
        ..Default::default()
    }).insert(state.conn).await.unwrap();

    
}