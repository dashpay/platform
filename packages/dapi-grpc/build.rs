use std::{
    collections::HashMap,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};

fn main() {
    generate().expect("failed to compile protobuf definitions");

    println!("cargo:rerun-if-changed=./protos");
}

/// Generate Rust definitions from Protobuf definitions
pub fn generate() -> Result<(), std::io::Error> {
    // Mapping between protobuf files => output directory
    let mut input = HashMap::<PathBuf, PathBuf>::new();
    input.insert(
        PathBuf::from("protos/core/v0/core.proto"),
        PathBuf::from("src/core/proto"),
    );
    input.insert(
        PathBuf::from("protos/platform/v0/platform.proto"),
        PathBuf::from("src/platform/proto"),
    );

    let proto_includes = vec![abs_path(&PathBuf::from("protos"))];

    for (proto, dest) in input {
        let proto = abs_path(&proto);
        let dest = abs_path(&dest);
        // Remove old compiled files; ignore errors
        if dest.exists() {
            remove_dir_all(&dest)?;
        }
        create_dir_all(&dest)?;

        generate1(&[proto], &proto_includes, &dest)?;
    }

    Ok(())
}

/// Run single generation process.
///
/// All paths must be absolute
fn generate1(
    files: &[PathBuf],
    proto_includes: &[PathBuf],
    out_dir: &PathBuf,
) -> Result<(), std::io::Error> {
    let pb = tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .protoc_arg("--experimental_allow_proto3_optional");
    #[cfg(feature = "client")]
    let pb = pb.build_client(true).build_transport(true);
    #[cfg(not(feature = "client"))]
    let pb = pb.build_client(false).build_transport(false);

    pb.compile(files, proto_includes)
}

fn abs_path(path: &PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}
