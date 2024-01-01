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

#[derive(Default)]
pub struct CaseRunner<S: Send + Sync + Default> {
    case: Vec<Box<dyn Runable<S> + Send + Sync + 'static>>,
    state: S,
}

impl<S: Send + Sync + Default> CaseRunner<S> {
    pub fn add_case<Rhs: Case<S> + Send + Sync + 'static>(&mut self, case: Rhs) {
        self.case.push(Box::new(case));
    }
    pub async fn run(mut self, title: &'static str) -> S {
        log::info!("Start testsuit {}", title);
        for (i, case) in self.case.into_iter().enumerate() {
            log::info!("Running case {} {}", i, case.name());
            if let Err(err) = case.run(&mut self.state).await {
                log::error!("test fail: {}", err);
                break;
            }
        }
        log::info!("End");

        self.state
    }
}
