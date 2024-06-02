fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../grpc/proto/backend.proto")?;
    Ok(())
}
