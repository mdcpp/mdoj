use uuid::Uuid;

use crate::{grpc::prelude::JudgeMatchRule, init::config::CONFIG, langs::prelude::ArtifactFactory};

async fn test_hello_world(factory: &mut ArtifactFactory, uuid: Uuid, code: &[u8]) {
    let mut compiled = factory.compile(&uuid, code).await.unwrap();

    let mut result = compiled
        .judge(b"", 1000 * 1000, 1024 * 1024 * 128)
        .await
        .unwrap();

    assert!(result.assert(b"hello world", JudgeMatchRule::SkipSnl));
}
#[tokio::test]
async fn built_in_plugin() {
    crate::init::new().await;

    let config = CONFIG.get().unwrap();
    let mut factory = ArtifactFactory::default();

    factory.load_dir(config.plugin.path.clone()).await;

    // lua
    test_hello_world(
        &mut factory,
        Uuid::parse_str("1c41598f-e253-4f81-9ef5-d50bf1e4e74f").unwrap(),
        b"print(\"hello world\")",
    )
    .await;
    // cpp
    test_hello_world(
        &mut factory,
        Uuid::parse_str("8a9e1daf-ff89-42c3-b011-bf6fb4bd8b26").unwrap(),
        b"#include <stdio.h>\nint main(){printf(\"hello world\");return 0;}",
    )
    .await;
    // c
    test_hello_world(
        &mut factory,
        Uuid::parse_str("7daff707-26b5-4153-90ae-9858b9fd9619").unwrap(),
        b"#include <stdio.h>\nint main(){printf(\"hello world\");return 0;}",
    )
    .await;
}
