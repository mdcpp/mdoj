use crate::util::auth::Auth;
use sea_orm::DatabaseConnection;

pub struct WithAuth<'a, T>(pub &'a Auth, pub T);
pub struct WithDB<'a, T>(pub &'a DatabaseConnection, pub T);

pub trait WithAuthTrait
where
    Self: Sized,
{
    fn with_auth(self, auth: &Auth) -> WithAuth<Self> {
        WithAuth(auth, self)
    }
}

pub trait WithDBTrait
where
    Self: Sized,
{
    fn with_db(self, db: &DatabaseConnection) -> WithDB<Self> {
        WithDB(db, self)
    }
}

impl<'a, 'b, T> WithDB<'a, WithAuth<'b, T>> {
    pub fn into_inner(self) -> T {
        self.1 .1
    }
}

impl<T: Sized> WithDBTrait for WithAuth<'_, T> {}
