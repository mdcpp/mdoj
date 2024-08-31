use anyhow::Result;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub data: ProblemData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problems {
    pub data: ProblemsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemsData {
    pub results: Vec<ProblemData>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemData {
    pub id: u64,
    pub tags: Vec<String>,
    pub title: String,
    pub description: String,
    pub input_description: String,
    pub output_description: String,
    pub samples: Vec<Sample>,
    pub test_case_id: String,
    pub test_case_score: Vec<TestCaseScore>,
    pub hint: String,
    pub languages: Vec<String>,
    pub create_time: String,
    pub last_update_time: Option<serde_json::Value>,
    pub time_limit: u64,
    pub memory_limit: u64,
    pub io_mode: IoMode,
    pub rule_type: String,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Difficulty {
    Low,
    Mid,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoMode {
    pub input: String,
    pub output: String,
    pub io_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub input: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseScore {
    pub score: i64,
    pub input_name: String,
    pub output_name: String,
}

pub async fn problem(client: &Client, base_url: &Url, id: usize) -> Result<ProblemData> {
    let problem: Problem = client
        .get(base_url.join("admin/problem")?)
        .query(&[("id", id)])
        .send()
        .await?
        .json()
        .await?;
    Ok(problem.data)
}

pub async fn problems(client: &Client, base_url: &Url) -> Result<Vec<ProblemData>> {
    const PAGE_SIZE: u64 = 250;
    let mut ret = vec![];
    for i in 0.. {
        let problems: Problems = client
            .get(base_url.join("admin/problem")?)
            .query(&[("limit", PAGE_SIZE), ("offset", PAGE_SIZE * i)])
            .send()
            .await?
            .json()
            .await?;

        ret.extend(problems.data.results);
        if problems.data.total <= PAGE_SIZE * i {
            break;
        }
    }
    Ok(ret)
}
