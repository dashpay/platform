use std::path::Path;
use std::process::Command;
use std::{env, fs};

fn main() {
    let crate_name = env::var("CARGO_PKG_NAME").unwrap();
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/");

    let commit = run_git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=PLATFORM_WALLET_GIT_COMMIT={commit}");

    let dirty = match run_git(&["status", "--porcelain"]) {
        Some(out) if !out.is_empty() => "1",
        Some(_) => "0",
        None => "unknown",
    };
    println!("cargo:rustc-env=PLATFORM_WALLET_GIT_DIRTY={dirty}");

    if let Some(head) = run_git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(index) = run_git(&["rev-parse", "--git-path", "index"]) {
        println!("cargo:rerun-if-changed={index}");
    }

    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3) // This line moves up to the target/<PROFILE> directory
        .expect("Failed to find target dir");

    let include_dir = target_dir.join("include").join(&crate_name);

    fs::create_dir_all(&include_dir).unwrap();

    let output_path = include_dir.join(format!("{}.h", &crate_name));

    let config_path = Path::new(&crate_dir).join("cbindgen.toml");
    let config = cbindgen::Config::from_file(&config_path).expect("Failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&output_path);
}

fn run_git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
}
