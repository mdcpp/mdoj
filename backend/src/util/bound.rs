use super::error::Error;
use grpc::backend::*;
use tracing::instrument;

pub trait BoundCheck {
    /// return true if fail
    fn check(&self) -> bool;
    #[instrument(skip_all, level = "info")]
    fn bound_check(&self) -> Result<(), tonic::Status> {
        if self.check() {
            tracing::warn!(msg = "bound check fail");
            Err(Error::NumberTooLarge.into())
        } else {
            Ok(())
        }
    }
}

macro_rules! impl_basic_bound_check {
    ($n:ident) => {
        paste::paste! {
            impl BoundCheck for [<Create $n Request>] {
                fn check(&self) -> bool {
                    self.info.content.len() > 128 * 1024 || self.info.title.len() > 128
                }
            }
            impl BoundCheck for [<Update $n Request>] {
                fn check(&self) -> bool {
                    self.info
                        .content
                        .as_ref()
                        .map(String::len)
                        .unwrap_or_default()
                        > 128 * 1024
                        || self
                            .info
                            .title
                            .as_ref()
                            .map(String::len)
                            .unwrap_or_default()
                            > 128
                }
            }
        }
    };
}

impl_basic_bound_check!(Announcement);
impl_basic_bound_check!(Education);

macro_rules! list_request {
    ($n:ident) => {
        paste::paste! {
            impl BoundCheck for [<List $n Request>] {
                fn check(&self) -> bool {
                    if let Some(x) = &self.request {
                        (match x {
                            [<list_ $n:lower _request>]::Request::Create(x) => x
                                .query
                                .as_ref()
                                .map(|x| x.text.as_ref().map(String::len))
                                .flatten()
                                .unwrap_or_default(),
                            [<list_ $n:lower _request>]::Request::Paginator(x) => x.len(),
                        } > 512)
                    } else {
                        false
                    }
                }
            }
        }
    };
}

list_request!(Problem);
list_request!(Announcement);
list_request!(Contest);
list_request!(Education);
list_request!(User);

impl BoundCheck for ListChatRequest {
    fn check(&self) -> bool {
        self.offset > 4096 || self.size > 128
    }
}
impl BoundCheck for ListSubmitRequest {
    fn check(&self) -> bool {
        if let Some(x) = &self.request {
            (match x {
                list_submit_request::Request::Create(x) => 0,
                list_submit_request::Request::Paginator(x) => x.len(),
            } > 512)
        } else {
            false
        }
    }
}

impl BoundCheck for CreateChatRequest {
    fn check(&self) -> bool {
        self.message.len() > 8 * 1024
    }
}

impl BoundCheck for CreateContestRequest {
    fn check(&self) -> bool {
        self.info.title.len() > 128
            || self.info.tags.len() > 1024
            || self.info.content.len() > 128 * 1024
            || self
                .info
                .password
                .as_ref()
                .map(String::len)
                .unwrap_or_default()
                > 256
    }
}
impl BoundCheck for UpdateContestRequest {
    fn check(&self) -> bool {
        self.info
            .title
            .as_ref()
            .map(String::len)
            .unwrap_or_default()
            > 128
            || self.info.tags.as_ref().map(String::len).unwrap_or_default() > 1024
            || self
                .info
                .content
                .as_ref()
                .map(String::len)
                .unwrap_or_default()
                > 128 * 1024
            || self
                .info
                .password
                .as_ref()
                .map(String::len)
                .unwrap_or_default()
                > 256
    }
}

impl BoundCheck for CreateProblemRequest {
    fn check(&self) -> bool {
        self.info.title.len() > 128
            || self.info.tags.len() > 1024
            || self.info.content.len() > 128 * 1024
    }
}
impl BoundCheck for UpdateProblemRequest {
    fn check(&self) -> bool {
        self.info
            .title
            .as_ref()
            .map(String::len)
            .unwrap_or_default()
            > 128
            || self.info.tags.as_ref().map(String::len).unwrap_or_default() > 1024
            || self
                .info
                .content
                .as_ref()
                .map(String::len)
                .unwrap_or_default()
                > 128 * 1024
    }
}

impl BoundCheck for CreateSubmitRequest {
    fn check(&self) -> bool {
        self.code.len() > 64 * 1024
    }
}

impl BoundCheck for CreateTestcaseRequest {
    fn check(&self) -> bool {
        self.info.input.len() > 256 * 1024 || self.info.output.len() > 256 * 1024
    }
}

impl BoundCheck for UpdateTestcaseRequest {
    fn check(&self) -> bool {
        self.info.input.as_ref().map(Vec::len).unwrap_or_default() > 256 * 1024
            || self.info.output.as_ref().map(Vec::len).unwrap_or_default() > 256 * 1024
    }
}

impl BoundCheck for CreateUserRequest {
    fn check(&self) -> bool {
        self.info.username.len() > 256 || self.info.password.len() > 256
    }
}

impl BoundCheck for UpdateUserRequest {
    fn check(&self) -> bool {
        self.info
            .username
            .as_ref()
            .map(String::len)
            .unwrap_or_default()
            > 256
            || self
                .info
                .password
                .as_ref()
                .map(String::len)
                .unwrap_or_default()
                > 256
    }
}
