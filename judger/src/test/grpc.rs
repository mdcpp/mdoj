mod test {
    use std::sync::Arc;

    use tempfile::NamedTempFile;
    use tokio::net;
    use tonic::transport::{Endpoint, Server, Uri};
    use tower::service_fn;

    use crate::grpc::proto::prelude::{
        judge_response::Task, judger_client::JudgerClient, judger_server::JudgerServer, *,
    };
    use crate::grpc::server::GRpcServer;
    use crate::init;

    // TODO!: split test
    #[ignore = "it take very long time"]
    #[tokio::test]
    async fn full() {
        init::new().await;

        // create stub for unix socket
        let server = Server::builder().add_service(JudgerServer::new(GRpcServer::new().await));
        let socket1 = Arc::new(NamedTempFile::new().unwrap().into_temp_path());
        let socket2 = socket1.clone();
        let socket3 = socket2.clone();

        // server thread(g)
        let server = tokio::spawn(async move {
            let uds = net::UnixListener::bind(&*socket2.clone()).unwrap();
            server
                .serve_with_incoming(tokio_stream::wrappers::UnixListenerStream::new(uds))
                .await
                .unwrap();
        });

        let channel = Endpoint::try_from("http://any.url")
            .unwrap()
            .connect_with_connector(service_fn(move |_: Uri| {
                let socket = Arc::clone(&socket1);
                async move { net::UnixStream::connect(&*socket).await }
            }))
            .await
            .unwrap();

        let mut client = JudgerClient::new(channel);

        let request = JudgeRequest {
            lang_uid: "f060f3c5-b2b2-46be-97ba-a128e5922aee".to_string(),
            code: b"print(\"basic test\")".to_vec(),
            memory: 1024 * 1024 * 1024,
            time: 1 * 1000 * 1000,
            rule: JudgeMatchRule::SkipSnl as i32,
            tests: vec![TestIo {
                input: b"".to_vec(),
                output: b"basic test".to_vec(),
            }],
        };

        let (_, mut res, _) = client.judge(request).await.unwrap().into_parts();

        // first request indicate test 1 start
        let res1 = res.message().await.unwrap().unwrap().task;
        assert_eq!(res1, Some(Task::Case(1)));

        let res2 = res.message().await.unwrap().unwrap().task;
        match res2.unwrap() {
            Task::Case(_) => panic!("expect Result"),
            Task::Result(result) => {
                assert_eq!(result.status, JudgeResultState::Ac as i32);
            }
        }
        server.abort();
        std::fs::remove_file(&*socket3).unwrap();
    }
}
