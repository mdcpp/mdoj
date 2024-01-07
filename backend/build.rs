fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .type_attribute(
            "oj.backend.SortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile(&["../proto/backend.proto"], &["../proto"])?;
    // tonic_build::compile_protos("../proto/backend.proto")?;
    // tonic_build::compile_protos("../proto/judger.proto")?;
    tonic_build::configure()
        .build_server(false)
        .type_attribute(
            "oj.backend.SortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile(&["../proto/judger.proto"], &["../proto"])?;
    Ok(())
}
