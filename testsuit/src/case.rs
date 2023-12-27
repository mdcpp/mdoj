use tonic::async_trait;

#[async_trait]
pub trait Case<S: Send + Sync> {
    const NAME: &'static str;
    async fn run(&self, state: &mut S) -> Result<(), String>;
}

#[async_trait]
trait Runable<S: Send + Sync> {
    async fn run(&self, state: &mut S) -> Result<(), String>;
    fn name(&self) -> &'static str;
}

#[async_trait]
impl<S: Send + Sync, Rhs: Case<S> + Send + Sync + 'static> Runable<S> for Rhs {
    async fn run(&self, state: &mut S) -> Result<(), String> {
        <Self as Case<S>>::run(self, state).await
    }

    fn name(&self) -> &'static str {
        <Self as Case<S>>::NAME
    }
}

pub struct CaseRunner<S: Send + Sync> {
    case: Vec<Box<dyn Runable<S> + Send + Sync + 'static>>,
    state: S,
}

impl<S: Send + Sync> CaseRunner<S> {
    pub async fn new(state: S) -> Self {
        Self {
            case: Vec::new(),
            state,
        }
    }
    pub fn add_case<Rhs: Case<S> + Send + Sync + 'static>(&mut self, case: Rhs) {
        self.case.push(Box::new(case));
    }
    pub async fn run(mut self) {
        log::info!("Start testsuit");
        for (i, case) in self.case.into_iter().enumerate() {
            log::info!("Running case {} {}", i, case.name());
            case.run(&mut self.state).await.unwrap();
        }
        log::info!("End");
    }
}
