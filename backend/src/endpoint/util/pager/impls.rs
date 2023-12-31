use ::entity::*;
use sea_orm::*;

use crate::{grpc::backend::SortBy, init::db::DB};

use super::{HasParent, NoParent, PagerTrait, ParentalTrait};
use crate::endpoint::util::{auth::Auth, error::Error, filter::Filter};

#[tonic::async_trait]
impl ParentalTrait<contest::Entity> for HasParent<contest::Entity> {
    const COL_ID: contest::Column = contest::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<contest::Entity>, Error> {
        let db = DB.get().unwrap();

        Ok(auth.get_user(db).await?.find_related(contest::Entity))
    }
}

#[tonic::async_trait]
impl PagerTrait for problem::Entity {
    const TYPE_NUMBER: i32 = 1591223;
    const COL_ID: problem::Column = problem::Column::Id;
    const COL_TEXT: &'static [problem::Column] = &[problem::Column::Title, problem::Column::Tags];
    const COL_SELECT: &'static [problem::Column] = &[
        problem::Column::Id,
        problem::Column::Title,
        problem::Column::AcRate,
        problem::Column::SubmitCount,
        problem::Column::Difficulty,
    ];

    type ParentMarker = HasParent<contest::Entity>;

    fn sort_column(sort: &SortBy) -> problem::Column {
        match sort {
            SortBy::UploadDate => problem::Column::UpdateAt,
            SortBy::CreateDate => problem::Column::CreateAt,
            SortBy::AcRate => problem::Column::AcRate,
            SortBy::SubmitCount => problem::Column::SubmitCount,
            SortBy::Difficulty => problem::Column::Difficulty,
            _ => problem::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::UploadDate => model.update_at.to_string(),
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::AcRate => model.ac_rate.to_string(),
            SortBy::SubmitCount => model.submit_count.to_string(),
            SortBy::Difficulty => model.difficulty.to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        problem::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl ParentalTrait<problem::Entity> for HasParent<problem::Entity> {
    const COL_ID: problem::Column = problem::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<problem::Entity>, Error> {
        let db = DB.get().unwrap();

        Ok(auth.get_user(db).await?.find_related(problem::Entity))
    }
}

#[tonic::async_trait]
impl PagerTrait for test::Entity {
    const TYPE_NUMBER: i32 = 78879091;
    const COL_ID: Self::Column = test::Column::Id;
    const COL_TEXT: &'static [Self::Column] = &[test::Column::Output, test::Column::Input];
    const COL_SELECT: &'static [Self::Column] = &[
        test::Column::Id,
        test::Column::UserId,
        test::Column::ProblemId,
    ];

    type ParentMarker = HasParent<problem::Entity>;

    fn sort_column(sort: &SortBy) -> test::Column {
        match sort {
            SortBy::Score => test::Column::Score,
            _ => test::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::Score => (model.score).to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        test::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for contest::Entity {
    const TYPE_NUMBER: i32 = 61475758;
    const COL_ID: Self::Column = contest::Column::Id;
    const COL_TEXT: &'static [Self::Column] = &[contest::Column::Title, contest::Column::Tags];
    const COL_SELECT: &'static [Self::Column] = &[
        contest::Column::Id,
        contest::Column::Title,
        contest::Column::Begin,
        contest::Column::End,
        contest::Column::Hoster,
    ];

    type ParentMarker = NoParent;

    fn sort_column(sort: &SortBy) -> contest::Column {
        match sort {
            SortBy::CreateDate => contest::Column::CreateAt,
            SortBy::UploadDate => contest::Column::UpdateAt,
            SortBy::Begin => contest::Column::Begin,
            SortBy::End => contest::Column::End,
            _ => contest::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::UploadDate => model.update_at.to_string(),
            SortBy::Begin => model.begin.to_string(),
            SortBy::End => model.end.to_string(),
            _ => model.id.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        contest::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for user::Entity {
    const TYPE_NUMBER: i32 = 1929833;

    const COL_ID: Self::Column = user::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[user::Column::Username];

    const COL_SELECT: &'static [Self::Column] = &[
        user::Column::Id,
        user::Column::Username,
        user::Column::Permission,
        user::Column::CreateAt,
    ];

    type ParentMarker = NoParent;

    fn sort_column(sort: &SortBy) -> user::Column {
        match sort {
            SortBy::CreateDate => user::Column::CreateAt,
            SortBy::Score => user::Column::Score,
            _ => user::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::CreateDate => model.create_at.to_string(),
            SortBy::Score => model.score.to_string(),
            _ => model.id.to_string(),
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        user::Entity::read_filter(select, auth)
    }
}
#[tonic::async_trait]
impl PagerTrait for submit::Entity {
    const TYPE_NUMBER: i32 = 539267;

    const COL_ID: Self::Column = submit::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[submit::Column::Id];

    const COL_SELECT: &'static [Self::Column] = &[
        submit::Column::Committed,
        submit::Column::Id,
        submit::Column::Time,
        submit::Column::Memory,
        submit::Column::PassCase,
        submit::Column::UploadAt,
    ];

    type ParentMarker = HasParent<problem::Entity>;

    fn sort_column(sort: &SortBy) -> submit::Column {
        match sort {
            SortBy::Committed => submit::Column::Committed,
            SortBy::Score => submit::Column::Score,
            SortBy::Time => submit::Column::Time,
            SortBy::Memory => submit::Column::Memory,
            SortBy::UploadDate | SortBy::CreateDate => submit::Column::UploadAt,
            _ => submit::Column::Id,
        }
    }
    fn get_key_of(model: &Self::Model, sort: &SortBy) -> String {
        match sort {
            SortBy::Committed => match model.committed {
                true => "1".to_string(),
                false => "0".to_string(),
            },
            SortBy::Score => model.score.to_string(),
            SortBy::Time => model.time.unwrap_or_default().to_string(),
            SortBy::Memory => model.memory.unwrap_or_default().to_string(),
            SortBy::UploadDate | SortBy::CreateDate => model.upload_at.to_string(),
            _ => model.id.to_string(),
        }
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        submit::Entity::read_filter(select, auth)
    }
}

#[tonic::async_trait]
impl PagerTrait for education::Entity {
    const TYPE_NUMBER: i32 = 183456;

    const COL_ID: Self::Column = education::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[education::Column::Title];

    const COL_SELECT: &'static [Self::Column] = &[education::Column::Id, education::Column::Title];

    type ParentMarker = HasParent<problem::Entity>;

    fn sort_column(_sort: &SortBy) -> education::Column {
        education::Column::Id
    }
    fn get_key_of(model: &Self::Model, _sort: &SortBy) -> String {
        model.id.to_string()
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        education::Entity::read_filter(select, auth)
    }
}

impl PagerTrait for chat::Entity {
    const TYPE_NUMBER: i32 = 3278361;

    const COL_ID: Self::Column = chat::Column::Id;

    const COL_TEXT: &'static [Self::Column] = &[chat::Column::Message];

    const COL_SELECT: &'static [Self::Column] = &[chat::Column::Id, chat::Column::Message];

    type ParentMarker = HasParent<problem::Entity>;

    fn get_key_of(model: &Self::Model, _sort: &SortBy) -> String {
        model.id.to_string()
    }

    fn sort_column(_sort: &SortBy) -> Self::Column {
        chat::Column::Id
    }

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }

    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        chat::Entity::read_filter(select, auth)
    }
}
