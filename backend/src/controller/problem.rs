use crate::{common::JudgeStatus, grpc::proto::prelude::judge_response};
use chrono::Utc;
use entity::{problem, submit};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::watch;
use tokio_stream::StreamExt;

use crate::init::db::DB;

use super::{util::router, Error};

pub struct ProblemBase {
    pub title: String,
    pub owner: i32,
}

// pub struct ProblemUpdate{
//     title:Option<String>,
//     description:Option<String>,
// }
pub enum Status {
    Running(i32),
    End(JudgeStatus),
}

pub struct ProblemController {
    judgers: router::JudgeRouter,
    running_submits: Mutex<HashMap<i32, watch::Receiver<Status>>>,
}

macro_rules! report {
    ($issue:expr) => {
        match $issue {
            Ok(x) => x,
            Err(err) => {
                log::error!("{}", err);
                return;
            }
        }
    };
}

impl ProblemController {
    // "add" essentially means "create", right? 
    pub async fn add(&self, base: ProblemBase) -> Result<i32, Error> {
        let db = DB.get().unwrap();

        let problem = problem::ActiveModel {
            title: ActiveValue::Set(base.title),
            user_id: ActiveValue::Set(base.owner),
            ..Default::default()
        }
        .insert(db)
        .await?;

        // where is Tasks::from_raw()

        Ok(problem.id)
    }
    // where is the input(args)?
    pub async fn update(&self,problem_id:i32) -> Result<(), Error>{
        


        todo!()
    }
    pub async fn remove(&self, problem_id: i32) -> Result<Option<()>, Error> {
        let db = DB.get().unwrap();

        let problem = problem::Entity::delete_many()
            .filter(problem::Column::Id.eq(problem_id))
            .exec(db)
            .await?;

        Ok(match problem.rows_affected == 0 {
            true => Some(()),
            false => None,
        })
    }
    pub async fn submit(
        &self,
        problem: problem::Model,
        code: Vec<u8>,
        user_id: i32,
        lang: String,
    ) -> Result<i32, Error> {
        let db = DB.get().unwrap();
        let now = Utc::now().naive_utc();

        let submit = submit::ActiveModel {
            user_id: ActiveValue::Set(user_id),
            problem_id: ActiveValue::Set(problem.id),
            upload: ActiveValue::Set(now),
            code: ActiveValue::Set(code),
            lang: ActiveValue::Set(lang),
            ..Default::default()
        }
        .save(db)
        .await?;

        let mut stream = self
            .judgers
            .route(
                problem,
                submit.code.as_ref().clone(),
                submit.lang.as_ref().clone(),
            )
            .await?;

        let (tx, rx) = watch::channel(Status::Running(1));
        {
            self.running_submits
                .lock()
                .unwrap()
                .insert(*submit.id.as_ref(), rx);
        }

        let submit_id = *submit.id.as_ref();

        tokio::spawn(async move {
            let mut max_time = 0;
            while let Some(res) = stream.next().await {
                if let Err(err) = res {
                    log::error!("Error from judger: {}", err);
                    break;
                } else if res.as_ref().unwrap().task.is_none() {
                    break;
                }
                match res.unwrap().task.unwrap() {
                    judge_response::Task::Case(case) => {
                        tx.send(Status::Running(case)).ok();
                    }
                    judge_response::Task::Result(x) => {
                        let exit = JudgeStatus::from_i32(x.status);
                        if !exit.success() {
                            max_time += x.max_time.expect(
                                "incorrect proto impl, expecting max_time persented when AC",
                            );
                            tx.send(Status::End(exit)).ok();
                        }
                    }
                };
            }

            todo!("use transcation to commit the submit");
            let model = report!(submit::Entity::find_by_id(submit_id).one(db).await);
            let model = report!(model.ok_or(Error::NotFound("Uncommited submit")));
        });
        self.running_submits.lock().unwrap().remove(&submit_id);

        Ok(submit_id)
    }
    pub async fn trace_submit(&self, submit_id: i32) -> Option<watch::Receiver<Status>> {
        self.running_submits
            .lock()
            .unwrap()
            .get(&submit_id)
            .map(|x| x.clone())
    }
    // pub async fn update()->
}
