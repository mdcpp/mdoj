use ::entity::*;
use sea_orm::*;

use crate::{
    grpc::backend::{
        AnnouncementSortBy, ContestSortBy, ProblemSortBy, SubmitSortBy, TestcaseSortBy, UserSortBy,
    },
    init::db::DB,
};

use super::{EmptySortBy, HasParent, NoParent, PagerTrait, ParentalTrait};
use crate::util::{auth::Auth, error::Error, filter::Filter};

#[tonic::async_trait]
impl ParentalTrait<contest::Entity> for HasParent<contest::Entity> {
    const COL_ID: contest::Column = contest::Column::Id;

    async fn related_filter(auth: &Auth) -> Result<Select<contest::Entity>, Error> {
        let db = DB.get().unwrap();
        // FIXME: write join
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
    const DEBUG_NAME: &'static str = "problem";

    type ParentMarker = HasParent<contest::Entity>;
    type SortBy = ProblemSortBy;

    fn sort_column(sort: &ProblemSortBy) -> problem::Column {
        match sort {
            ProblemSortBy::UploadDate => problem::Column::UpdateAt,
            ProblemSortBy::CreateDate => problem::Column::CreateAt,
            ProblemSortBy::AcRate => problem::Column::AcRate,
            ProblemSortBy::SubmitCount => problem::Column::SubmitCount,
            ProblemSortBy::Difficulty => problem::Column::Difficulty,
            ProblemSortBy::Order => problem::Column::Order,
        }
    }
    fn sort_value(model: &Self::Model, sort: &ProblemSortBy) -> String {
        match sort {
            ProblemSortBy::UploadDate => model.update_at.to_string(),
            ProblemSortBy::CreateDate => model.create_at.to_string(),
            ProblemSortBy::AcRate => model.ac_rate.to_string(),
            ProblemSortBy::SubmitCount => model.submit_count.to_string(),
            ProblemSortBy::Difficulty => model.difficulty.to_string(),
            ProblemSortBy::Order => model.order.to_string(),
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
impl PagerTrait for announcement::Entity {
    const TYPE_NUMBER: i32 = 1591223;
    const COL_ID: announcement::Column = announcement::Column::Id;
    const COL_TEXT: &'static [announcement::Column] = &[announcement::Column::Title];
    const COL_SELECT: &'static [announcement::Column] =
        &[announcement::Column::Id, announcement::Column::Title];
    const DEBUG_NAME: &'static str = "announcement";

    type ParentMarker = HasParent<contest::Entity>;
    type SortBy = AnnouncementSortBy;

    fn sort_column(sort: &AnnouncementSortBy) -> announcement::Column {
        match sort {
            AnnouncementSortBy::UploadDate => announcement::Column::UpdateAt,
            AnnouncementSortBy::CreateDate => announcement::Column::CreateAt,
        }
    }
    fn sort_value(model: &Self::Model, sort: &AnnouncementSortBy) -> String {
        match sort {
            AnnouncementSortBy::UploadDate => model.update_at.to_string(),
            AnnouncementSortBy::CreateDate => model.create_at.to_string(),
        }
    }
    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        announcement::Entity::read_filter(select, auth)
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
    const DEBUG_NAME: &'static str = "testcase";

    type ParentMarker = HasParent<problem::Entity>;
    type SortBy = TestcaseSortBy;

    fn sort_column(sort: &TestcaseSortBy) -> test::Column {
        match sort {
            TestcaseSortBy::Score => test::Column::Score,
        }
    }
    fn sort_value(model: &Self::Model, sort: &TestcaseSortBy) -> String {
        match sort {
            TestcaseSortBy::Score => (model.score).to_string(),
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
    const DEBUG_NAME: &'static str = "contest";

    type ParentMarker = NoParent;
    type SortBy = ContestSortBy;

    fn sort_column(sort: &ContestSortBy) -> contest::Column {
        match sort {
            ContestSortBy::CreateDate => contest::Column::CreateAt,
            ContestSortBy::UploadDate => contest::Column::UpdateAt,
            ContestSortBy::Begin => contest::Column::Begin,
            ContestSortBy::End => contest::Column::End,
        }
    }
    fn sort_value(model: &Self::Model, sort: &ContestSortBy) -> String {
        match sort {
            ContestSortBy::CreateDate => model.create_at.to_string(),
            ContestSortBy::UploadDate => model.update_at.to_string(),
            ContestSortBy::Begin => model.begin.to_string(),
            ContestSortBy::End => model.end.to_string(),
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
    const DEBUG_NAME: &'static str = "user";

    type ParentMarker = NoParent;
    type SortBy = UserSortBy;

    fn sort_column(sort: &UserSortBy) -> user::Column {
        match sort {
            UserSortBy::CreateDate => user::Column::CreateAt,
            UserSortBy::Score => user::Column::Score,
        }
    }
    fn sort_value(model: &Self::Model, sort: &UserSortBy) -> String {
        match sort {
            UserSortBy::CreateDate => model.create_at.to_string(),
            UserSortBy::Score => model.score.to_string(),
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
    const DEBUG_NAME: &'static str = "submit";

    type ParentMarker = HasParent<problem::Entity>;
    type SortBy = SubmitSortBy;

    fn sort_column(sort: &SubmitSortBy) -> submit::Column {
        match sort {
            SubmitSortBy::Committed => submit::Column::Committed,
            SubmitSortBy::Score => submit::Column::Score,
            SubmitSortBy::Time => submit::Column::Time,
            SubmitSortBy::Memory => submit::Column::Memory,
            SubmitSortBy::UploadDate => submit::Column::UploadAt,
        }
    }
    fn sort_value(model: &Self::Model, sort: &SubmitSortBy) -> String {
        match sort {
            SubmitSortBy::Committed => match model.committed {
                true => "1".to_string(),
                false => "0".to_string(),
            },
            SubmitSortBy::Score => model.score.to_string(),
            SubmitSortBy::Time => model.time.unwrap_or_default().to_string(),
            SubmitSortBy::Memory => model.memory.unwrap_or_default().to_string(),
            SubmitSortBy::UploadDate => model.upload_at.to_string(),
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
    const DEBUG_NAME: &'static str = "education";

    type ParentMarker = HasParent<problem::Entity>;
    type SortBy = EmptySortBy;

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
    const DEBUG_NAME: &'static str = "chat";

    type ParentMarker = HasParent<problem::Entity>;
    type SortBy = EmptySortBy;

    fn get_id(model: &Self::Model) -> i32 {
        model.id
    }
    fn query_filter(select: Select<Self>, auth: &Auth) -> Result<Select<Self>, Error> {
        chat::Entity::read_filter(select, auth)
    }
}
