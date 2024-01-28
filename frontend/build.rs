fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        // .type_attribute(".", "#[derive(Serialize,Deserialize)]")
        .compile(&["../proto/backend.proto"], &["../proto/"])?;
    Ok(())
}
