use bincode;
use serde::{Deserialize, Serialize};


pub enum Role {
    Admin,
    User
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleData{
    create_problem:bool,
    create_contest:bool,
    create_user:bool,
    delete_problem:bool,
    delete_contest:bool,
    delete_user:bool,
}

impl Role{
    pub fn to_data(&self)->RoleData{
        match self{
            Role::Admin => RoleData{
                create_problem: true,
                create_contest: true,
                create_user: true,
                delete_problem: true,
                delete_contest: true,
                delete_user: true,
            },
            Role::User => RoleData{
                create_problem: false,
                create_contest:false,
                create_user: false,
                delete_problem: false,
                delete_contest: false,
                delete_user: false,
            },
        }
    }
    pub fn to_binary(&self)->Vec<u8>{
        bincode::serialize(&self.to_data()).unwrap()
    }
}
