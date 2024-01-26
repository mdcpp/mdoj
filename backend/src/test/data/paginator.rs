use crate::entity::*;
use sea_orm::{prelude::*, IntoActiveModel};
use tonic::async_trait;

pub struct PaginatorData;

#[async_trait]
impl super::Data for PaginatorData {
    async fn insert(db: &DatabaseConnection) {
        user::Model {
            id: 1,
            permission: 4,
            score: 0,
            username: todo!(),
            password: todo!(),
            create_at: todo!(),
        }
        .into_active_model()
        .save(db)
        .await
        .unwrap();
        problem::Model {
            id: 1,
            user_id: 2,
            contest_id: todo!(),
            accept_count: todo!(),
            submit_count: todo!(),
            ac_rate: todo!(),
            memory: todo!(),
            time: todo!(),
            difficulty: todo!(),
            public: todo!(),
            tags: todo!(),
            title: todo!(),
            content: todo!(),
            create_at: todo!(),
            update_at: todo!(),
            match_rule: todo!(),
            order: todo!(),
        };
    }
}
