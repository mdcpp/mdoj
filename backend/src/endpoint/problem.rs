use crate::controller::{problem, Controllers};

pub struct ProblemEndpoint<'a>(pub &'a Controllers);

pub struct Base {
    title: String,
}

impl problem::Base {
    fn from_request(require: Base, owner: i32) -> problem::Base {
        problem::Base {
            title: require.title,
            owner,
        }
    }
}

impl<'a> ProblemEndpoint<'a> {
    async fn create(&self, request: Base, user: UserInfo) -> Result<i32, super::Error> {
        if user.perm.can_manage_problem() {
            let problem = self
                .0
                .problem
                .create(problem::Base::from_request(request, user.user_id))
                .await?;
            Ok(problem)
        } else {
            Err(super::Error::PremissionDeny)
        }
    }

    async fn update(&self, request: problem::Update, user: UserInfo) -> Result<i32, super::Error> {
        todo!()
    }

    async fn remove(&self, request: i32, user: UserInfo) -> Result<i32, super::Error> {
        todo!()
    }
}
