#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unreachable")]
    Unreachable,
}

// pub struct StackedTestcase<O> where Self:Sized{
//     next:Box<dyn Testcase<O>>
// }

// impl<I, O> Testcase<I> for StackedTestcase<O> {

// }
// #[async_trait]
// pub trait Testcase<I,O>{
//     fn run_inner(&self,state:I)->Result<I,Error>{

//     }
//     fn stack(self,state:I,next:Box<dyn Testcase<O>>)->Result<O,Error>where O:From<I>;
// }
