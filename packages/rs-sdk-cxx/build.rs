use std::path::Path;
use std::{env, fs};

/// Generates the cxx bridge and stages the headers an embedder includes
/// under `target/<profile>/include/`, next to the static archive:
/// `dash/sdk/ffi.h` (the generated bridge), `dash/sdk/callbacks.h` (the
/// hand-written C++ callback types the bridge names) and `rust/cxx.h`.
fn main() {
    cxx_build::CFG.include_prefix = "dash/sdk";
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .std("c++20")
        .compile("dash-sdk-cxx-bridge");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=include/dash/sdk/callbacks.h");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let include = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("target/<profile>")
        .join("include");
    let generated = Path::new(&out_dir).join("cxxbridge/include");
    for (from, to) in [
        (generated.join("dash/sdk/src/lib.rs.h"), "dash/sdk/ffi.h"),
        (generated.join("rust/cxx.h"), "rust/cxx.h"),
        (
            Path::new("include/dash/sdk/callbacks.h").to_path_buf(),
            "dash/sdk/callbacks.h",
        ),
    ] {
        let to = include.join(to);
        fs::create_dir_all(to.parent().expect("parent")).expect("create include dir");
        fs::copy(&from, &to)
            .unwrap_or_else(|e| panic!("copy {} to {}: {e}", from.display(), to.display()));
    }
}
