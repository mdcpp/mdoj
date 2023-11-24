use uuid::Uuid;

use crate::{
    grpc::proto::prelude::JudgeMatchRule, init::config::CONFIG, langs::prelude::ArtifactFactory,
};

#[tokio::test]
async fn lua() {
    crate::init::new().await;

    let config = CONFIG.get().unwrap();
    let mut factory = ArtifactFactory::default();

    factory.load_dir(config.plugin.path.clone()).await;

    {
        let uuid = Uuid::parse_str("f060f3c5-b2b2-46be-97ba-a128e5922aee").unwrap();

        let mut compiled = factory
            .compile(&uuid, b"print(\"hello world\")")
            .await
            .unwrap();

        let result = compiled
            .judge(b"", 1000 * 1000, 1024 * 1024 * 128)
            .await
            .unwrap();

        assert!(result.assert(b"hello world", JudgeMatchRule::SkipSnl))
    };
    {
        let uuid = Uuid::parse_str("8a9e1daf-ff89-42c3-b011-bf6fb4bd8b26").unwrap();

        let mut compiled = factory
            .compile(
                &uuid,
                b"#include <stdio.h>\nint main(){printf(\"hello world\");return 0;}",
            )
            .await
            .unwrap();

        let result = compiled
            .judge(b"", 1000 * 1000, 1024 * 1024 * 128)
            .await
            .unwrap();

        assert!(result.assert(b"hello world", JudgeMatchRule::SkipSnl));
    };
}
