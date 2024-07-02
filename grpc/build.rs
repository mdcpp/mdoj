use std::{env, path::PathBuf};
use tonic_build::configure;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_file = out.join("descriptors.bin");
    configure()
        .build_server(cfg!(feature = "server"))
        .build_client(cfg!(feature = "client"))
        .type_attribute(
            ".",
            r#"#[cfg_attr(feature = "serde", derive(serde::Serialize,serde::Deserialize))]"#,
        )
        .extern_path(".google.protobuf.Any", "::prost_wkt_types::Any")
        .extern_path(".google.protobuf.Timestamp", "::prost_wkt_types::Timestamp")
        .extern_path(".google.protobuf.Value", "::prost_wkt_types::Value")
        .file_descriptor_set_path(&descriptor_file)
        .compile(
            &[
                #[cfg(feature = "backend")]
                "proto/backend.proto",
                #[cfg(feature = "judger")]
                "proto/judger.proto",
            ],
            &["proto"],
        )
        .unwrap();

    #[cfg(feature = "serde")]
    {
        use prost_wkt_build::*;
        let descriptor_bytes = std::fs::read(descriptor_file).unwrap();
        let descriptor = FileDescriptorSet::decode(&descriptor_bytes[..]).unwrap();
        prost_wkt_build::add_serde(out, descriptor);
    }
}
