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
    // `OUT_DIR` is `target/<profile>/build/<package>-<hash>/out` up to Cargo 1.99 and
    // `target/<profile>/build/<package>/<hash>/out` from Cargo 1.100, so the profile
    // directory is the parent of `build`, whatever lies between `build` and `out`.
    let include = Path::new(&out_dir)
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(Path::parent)
        .expect("OUT_DIR lies under target/<profile>/build")
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
