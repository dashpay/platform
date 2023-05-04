mod generate {
    include!("src/generate.rs");
}

fn main() {
    generate::generate().expect("failed to compile protobuf definitions");

    println!("cargo:rerun-if-changed=./protos");
    println!("cargo:rerun-if-changed=./src/generate.rs");
}
