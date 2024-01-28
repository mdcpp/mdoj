fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .type_attribute(
            "oj.backend.SortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.SubmitSortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.ProblemSortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.TestcaseSortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.ContestSortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.UserSortBy",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .type_attribute(
            "oj.backend.AnnouncementSortBy",
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
