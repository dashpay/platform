use std::path::Path;
use std::{env, fs};

/// Generates the CXX bridge and stages every header an embedder includes
/// under `target/<profile>/include/`, the way the cbindgen-based FFI crates
/// in this workspace stage theirs (`rs-sdk-ffi`, `rs-platform-wallet-ffi`):
/// `dash/platform/ffi.h` (the generated bridge header), `rust/cxx.h` (the
/// cxx runtime it includes) and `dash/platform/signer.h` (the hand-written
/// callback type the bridge's `extern "C++"` block includes). Build systems
/// install that `include/` tree and the static archive; nothing else in the
/// crate directory is part of the interface.
fn main() {
    // Every bridge entry point converts panics into rust::Error through
    // catch_unwind; under panic=abort that protection is silently compiled
    // out and a panic aborts the embedding process. Refuse such a build.
    println!("cargo:rerun-if-env-changed=CARGO_CFG_PANIC");
    if env::var("CARGO_CFG_PANIC").as_deref() == Ok("abort") {
        panic!(
            "dash-platform-cxx requires panic = \"unwind\"; its FFI guards rely on catch_unwind"
        );
    }

    cxx_build::CFG.include_prefix = "dash/platform";
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .std("c++20")
        .compile("dash-platform-cxx-bridge");

    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=include/dash/platform/signer.h");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let target_dir = Path::new(&out_dir)
        .ancestors()
        .nth(3)
        .expect("target/<profile> directory");
    let include_dir = target_dir.join("include");
    let platform_dir = include_dir.join("dash").join("platform");
    let rust_dir = include_dir.join("rust");
    fs::create_dir_all(&platform_dir).expect("create include dir");
    fs::create_dir_all(&rust_dir).expect("create include dir");

    let generated = Path::new(&out_dir).join("cxxbridge");
    copy(
        &generated.join("include/dash/platform/src/lib.rs.h"),
        &platform_dir.join("ffi.h"),
    );
    copy(
        &generated.join("include/rust/cxx.h"),
        &rust_dir.join("cxx.h"),
    );
    copy(
        Path::new("include/dash/platform/signer.h"),
        &platform_dir.join("signer.h"),
    );
}

fn copy(from: &Path, to: &Path) {
    fs::copy(from, to)
        .unwrap_or_else(|e| panic!("copy {} to {}: {e}", from.display(), to.display()));
}
