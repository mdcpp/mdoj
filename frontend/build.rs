fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // .type_attribute(".", "#[derive(Serialize,Deserialize)]")
        .compile(&["../grpc/proto/backend.proto"], &["../grpc/proto/"])?;
    Ok(())
}
