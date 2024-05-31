use grpc::judger::LangInfo;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Raw {
    pub fs_limit: Option<u64>,
    pub file: String,
    pub info: String,
    pub extension: String,
    pub name: String,
    pub id: Uuid,
    pub compile: RawCompile,
    pub judge: RawJudge,
}

impl<'a> From<&'a Raw> for LangInfo {
    fn from(value: &'a Raw) -> Self {
        LangInfo {
            lang_uid: value.id.to_string(),
            lang_name: value.name.clone(),
            info: value.info.clone(),
            lang_ext: value.extension.clone(),
        }
    }
}

impl Raw {
    pub fn fill(&mut self) {
        if self.fs_limit.is_none() {
            self.fs_limit = Some(67108864);
        }
        self.compile.fill();
        self.judge.fill();
    }
}

#[derive(Deserialize)]
pub struct RawCompile {
    pub command: Vec<String>,
    pub kernel_mem: Option<u64>,
    pub memory: Option<u64>,
    pub user_mem: Option<u64>,
    pub rt_time: Option<u64>,
    pub cpu_time: Option<u64>,
    pub time: Option<u64>,
    pub output_limit: Option<u64>,
    pub walltime: Option<u64>,
}

impl RawCompile {
    fn fill(&mut self) {
        let template = Self::default();
        macro_rules! try_fill {
            ($f:ident) => {
                if self.$f.is_none(){
                    self.$f=template.$f;
                }
            };
            ($f:ident,$($e:ident),+) => {
                try_fill!($f);
                try_fill!($($e),+);
            }
        }
        try_fill!(
            kernel_mem,
            user_mem,
            rt_time,
            cpu_time,
            time,
            output_limit,
            walltime,
            memory
        );
    }
}

impl Default for RawCompile {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            kernel_mem: Some(268435456),
            memory: Some(268435456),
            user_mem: Some(8589934592),
            rt_time: Some(7e8 as u64),
            cpu_time: Some(10e9 as u64),
            time: Some(10e9 as u64),
            output_limit: Some(33554432),
            walltime: Some(260e9 as u64),
        }
    }
}

#[derive(Deserialize)]
pub struct RawJudge {
    pub command: Vec<String>,
    pub kernel_mem: Option<u64>,
    pub rt_time: Option<u64>,
    pub memory_multiplier: Option<f64>,
    pub cpu_multiplier: Option<f64>,
    pub walltime: Option<u64>,
    pub output: Option<u64>,
}

impl RawJudge {
    fn fill(&mut self) {
        let template = Self::default();
        macro_rules! try_fill {
            ($f:ident) => {
                if self.$f.is_none(){
                    self.$f=template.$f;
                }
            };
            ($f:ident,$($e:ident),+) => {
                try_fill!($f);
                try_fill!($($e),+);
            }
        }
        try_fill!(
            kernel_mem,
            rt_time,
            memory_multiplier,
            cpu_multiplier,
            walltime,
            output
        );
    }
}

impl Default for RawJudge {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            kernel_mem: Some(268435456),
            rt_time: Some(7e8 as u64),
            memory_multiplier: Some(1.0),
            cpu_multiplier: Some(1.0),
            walltime: Some(360e9 as u64),
            output: Some(1024 * 1024 * 16),
        }
    }
}
