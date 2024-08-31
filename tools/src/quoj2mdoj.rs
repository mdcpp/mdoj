use crate::{
    grpc::{self, WithToken},
    quoj,
};
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Quoj2mdoj {
    #[arg(long)]
    duoj_api: String,
    #[arg(long)]
    duoj_session: String,

    #[arg(long)]
    mdoj_api: String,
    #[arg(long)]
    mdoj_session: String,

    #[arg(long)]
    problem_id: Option<usize>,
}

pub async fn quoj2mdoj(v: Quoj2mdoj) -> Result<()> {
    let quoj_client = quoj::QuojClient::new(v.duoj_api, v.duoj_session)?;
    let mut mdoj_client = grpc::problem_client::ProblemClient::connect(v.mdoj_api.clone()).await?;
    let mut mdoj_testcase_client =
        grpc::testcase_client::TestcaseClient::connect(v.mdoj_api).await?;

    if let Some(problem_id) = v.problem_id {
        let p = quoj_client.problem(problem_id).await?;
        return problem(
            p,
            &v.mdoj_session,
            &quoj_client,
            &mut mdoj_client,
            &mut mdoj_testcase_client,
        )
        .await;
    }

    let ps = quoj_client.problems().await?;
    for p in ps {
        problem(
            p,
            &v.mdoj_session,
            &quoj_client,
            &mut mdoj_client,
            &mut mdoj_testcase_client,
        )
        .await?;
    }

    Ok(())
}

async fn problem(
    problem: quoj::problem::ProblemData,
    session: &str,
    quoj_client: &quoj::QuojClient,
    mdoj_client: &mut grpc::problem_client::ProblemClient<tonic::transport::Channel>,
    mdoj_testcase_client: &mut grpc::testcase_client::TestcaseClient<tonic::transport::Channel>,
) -> Result<()> {
    let id = mdoj_client
        .create(
            grpc::CreateProblemRequest {
                info: grpc::create_problem_request::Info {
                    title: problem.title,
                    difficulty: match problem.difficulty {
                        quoj::problem::Difficulty::Low => 1000,
                        quoj::problem::Difficulty::Mid => 2000,
                        quoj::problem::Difficulty::High => 3000,
                    },
                    time: problem.time_limit * 1000,
                    memory: problem.memory_limit * 1024 * 1024,
                    content: format!(
                        "{}\n\n## 輸入\n\n{}\n\n## 輸出\n\n{}\n\n## 提示\n\n{}",
                        problem.description,
                        problem.input_description,
                        problem.output_description,
                        problem.hint
                    ),
                    match_rule: grpc::MatchRule::MatchruleIgnoreSnl.into(),
                    order: 0.0,
                    tags: problem.tags,
                },
                request_id: None,
            }
            .with_token(session),
        )
        .await?
        .into_inner()
        .id;

    let mut testcases = quoj_client.testcases(problem.id).await?;

    for testcase in problem.test_case_score {
        let testcase_id = mdoj_testcase_client
            .create(
                grpc::CreateTestcaseRequest {
                    info: grpc::create_testcase_request::Info {
                        score: testcase.score as u32,
                        input: testcases.testcase(testcase.input_name)?,
                        output: testcases.testcase(testcase.output_name)?,
                    },
                    request_id: None,
                }
                .with_token(session),
            )
            .await?
            .into_inner()
            .id;
        mdoj_testcase_client
            .add_to_problem(
                grpc::AddTestcaseToProblemRequest {
                    testcase_id,
                    problem_id: id,
                    request_id: None,
                }
                .with_token(session),
            )
            .await?;
    }

    mdoj_client
        .publish(
            grpc::PublishRequest {
                id,
                request_id: None,
            }
            .with_token(session),
        )
        .await?;
    Ok(())
}
