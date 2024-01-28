pub mod add_to;
pub mod create;
pub mod empty;
pub mod operate;
pub mod ui;

use std::path::Path;

use async_std::fs;
use indicatif::*;
use serde::{Deserialize, Serialize};
use tonic::async_trait;

use self::{create::StartOfId, ui::UI};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("assert fail `{0}`")]
    AssertFail(&'static str),
    #[error("expect rpc to success: `{0}`")]
    Tonic(#[from] tonic::Status),
}

#[derive(Default, Serialize, Deserialize)]
pub struct State {
    pub step: u64,
    #[serde(skip_deserializing, skip_serializing)]
    pub bar: MultiProgress,
    pub admin_token: Option<crate::tests::empty::login::AdminToken>,
    pub problem: Option<StartOfId<10>>,
    pub testcase: Option<StartOfId<3>>,
    pub contest: Option<StartOfId<1>>,
    pub user: Option<StartOfId<2>>,
}
// all testcase was added to the first problem
// the second and third problem was added to the only contest

impl State {
    pub async fn load() -> Self {
        let path = Path::new(crate::constants::DATA_PATH);
        match path.exists() {
            true => {
                let raw = fs::read(path).await.unwrap();
                toml::from_str(&String::from_utf8_lossy(&raw)).unwrap()
            }
            false => State::default(),
        }
    }
    pub async fn save(self) {
        let path = Path::new(crate::constants::DATA_PATH);
        let raw = toml::to_string_pretty(&self).unwrap();
        fs::write(path, raw).await.unwrap();
    }
}

#[async_trait]
pub trait Test {
    type Error: std::error::Error;
    const NAME: &'static str;
    async fn run(state: &mut State) -> Result<(), Self::Error>;
}

pub async fn run(mut state: State) -> State {
    let mut ui = UI::new(&state.bar, 3);

    macro_rules! handle {
        ($cc:expr,$e:ident) => {
            if ($cc)==state.step{
                log::info!("step {}",state.step);
                ui.inc($e::Test::NAME);
                if let Err(err)=$e::Test::run(&mut state).await{
                    log::error!("Error at {}, test stop, progress saved!",err);
                    return state;
                }
                state.step+=1;
            }
        };
        ($cc:expr,$x:ident, $($y:ident),+)=>{
            handle!($cc,$x);
            handle!($cc+1,$($y),+);
        }
    }

    handle!(0, empty, create, add_to, operate);
    state.bar.clear().unwrap();
    state
}
