use std::path::Path;
use std::{env, fs};

fn main() {
    let crate_name = env::var("CARGO_PKG_NAME").unwrap();
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/");

    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3) // This line moves up to the target/<PROFILE> directory
        .expect("Failed to find target dir");

    let include_dir = target_dir.join("include").join(&crate_name);

    fs::create_dir_all(&include_dir).unwrap();

    let output_path = include_dir.join(format!("{}.h", crate_name));

    let config_path = Path::new(&crate_dir).join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path).expect("Failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&output_path);
}
