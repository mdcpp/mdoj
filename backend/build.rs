fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/backend.proto")?;
    tonic_build::compile_protos("../proto/sandbox.proto")?;
    Ok(())
}
