use super::{auth::Auth, error::Error};
use entity::*;
use sea_orm::{ColumnTrait, EntityTrait, PrimaryKeyTrait, QueryFilter, Select};

/// filter for Entity r/w
pub trait Filter
where
    Self: EntityTrait,
{
    fn read_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn write_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
        Err(Error::Unauthenticated)
    }
    fn read_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::read_filter(Self::find_by_id(id), auth)
    }
    fn write_by_id<T>(id: T, auth: &Auth) -> Result<Select<Self>, Error>
    where
        T: Into<<Self::PrimaryKey as PrimaryKeyTrait>::ValueType>,
    {
        Self::write_filter(Self::find_by_id(id), auth)
    }
}

impl Filter for chat::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_root() || perm.can_manage_chat() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
}

impl Filter for announcement::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(
                announcement::Column::Public
                    .eq(true)
                    .or(announcement::Column::UserId.eq(user_id)),
            ));
        }
        Ok(query.filter(announcement::Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_announcement() {
            return Ok(query.filter(announcement::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write announcement"))
    }
}

impl Filter for contest::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(
                contest::Column::Public
                    .eq(true)
                    .or(contest::Column::Hoster.eq(user_id)),
            ));
        }
        Ok(query.filter(contest::Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_contest() {
            return Ok(query.filter(contest::Column::Hoster.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write contest"))
    }
}

impl Filter for education::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(education::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't read education"))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_education() {
            return Ok(query.filter(education::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write education"))
    }
}

impl Filter for problem::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(
                problem::Column::Public
                    .eq(true)
                    .or(problem::Column::UserId.eq(user_id)),
            ));
        }
        Ok(query.filter(problem::Column::Public.eq(true)))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_problem() {
            return Ok(query.filter(problem::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write problem"))
    }
}

impl Filter for submit::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Some(perm) = auth.user_perm() {
            if perm.can_manage_submit() || perm.can_root() {
                return Ok(query);
            }
        }
        Err(Error::Unauthenticated)
    }
}

impl Filter for test::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        if let Ok((user_id, perm)) = auth.ok_or_default() {
            if perm.can_root() {
                return Ok(query);
            }
            return Ok(query.filter(test::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't read testcase"))
    }
    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() {
            return Ok(query);
        }
        if perm.can_manage_problem() {
            return Ok(query.filter(test::Column::UserId.eq(user_id)));
        }
        Err(Error::PermissionDeny("Can't write testcase"))
    }
}

impl Filter for user::Entity {
    fn read_filter<S: QueryFilter + Send>(query: S, _: &Auth) -> Result<S, Error> {
        Ok(query)
    }

    fn write_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
        let (user_id, perm) = auth.ok_or_default()?;
        if perm.can_root() || perm.can_manage_user() {
            return Ok(query);
        }
        Ok(query.filter(user::Column::Id.eq(user_id)))
    }
}

// /// filter related to across Entity relation
// pub trait ParentalFilter {
//     fn publish_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
//         Err(Error::Unauthenticated)
//     }
//     fn link_filter<S: QueryFilter + Send>(_: S, _: &Auth) -> Result<S, Error> {
//         Err(Error::Unauthenticated)
//     }
// }

// impl ParentalFilter for announcement::Entity {
//     fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() || perm.can_manage_announcement() {
//                 return Ok(query);
//             }
//         }
//         Err(Error::PermissionDeny("Can't publish education"))
//     }

//     fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() || perm.can_manage_announcement() {
//                 return Ok(query);
//             }
//         }
//         Err(Error::PermissionDeny("Can't link education"))
//     }
// }

// impl ParentalFilter for education::Entity {
//     fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() {
//                 return Ok(query);
//             }
//             if perm.can_publish() {
//                 let user_id = auth.user_id().unwrap();
//                 return Ok(query.filter(education::Column::UserId.eq(user_id)));
//             }
//         }
//         Err(Error::PermissionDeny("Can't publish education"))
//     }

//     fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() {
//                 return Ok(query);
//             }
//             if perm.can_link() {
//                 let user_id = auth.user_id().unwrap();
//                 return Ok(query.filter(education::Column::UserId.eq(user_id)));
//             }
//         }
//         Err(Error::PermissionDeny("Can't link education"))
//     }
// }

// impl ParentalFilter for problem::Entity {
//     fn publish_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() {
//                 return Ok(query);
//             }
//             if perm.can_publish() {
//                 let user_id = auth.user_id().unwrap();
//                 return Ok(query.filter(problem::Column::UserId.eq(user_id)));
//             }
//         }
//         Err(Error::PermissionDeny("Can't publish problem"))
//     }

//     fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() {
//                 return Ok(query);
//             }
//             if perm.can_link() {
//                 let user_id = auth.user_id().unwrap();
//                 return Ok(query.filter(problem::Column::UserId.eq(user_id)));
//             }
//         }
//         Err(Error::PermissionDeny("Can't link problem"))
//     }
// }

// impl ParentalFilter for test::Entity {
//     fn link_filter<S: QueryFilter + Send>(query: S, auth: &Auth) -> Result<S, Error> {
//         if let Some(perm) = auth.user_perm() {
//             if perm.can_root() {
//                 return Ok(query);
//             }
//             if perm.can_link() {
//                 let user_id = auth.user_id().unwrap();
//                 return Ok(query.filter(test::Column::UserId.eq(user_id)));
//             }
//         }
//         Err(Error::PermissionDeny("Can't link test"))
//     }
// }
