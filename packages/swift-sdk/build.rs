use cbindgen::Config;

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Generate C headers for Swift interop
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(Config::from_file("cbindgen.toml").unwrap_or_default())
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("generated/SwiftDashSDK.h");

    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=cbindgen.toml");
}