use std::env;
use std::path::Path;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Only generate bindings when explicitly requested
    if env::var("GENERATE_BINDINGS").is_ok() {
        let config = cbindgen::Config {
            language: cbindgen::Language::C,
            pragma_once: true,
            include_guard: Some("IOS_SDK_FFI_H".to_string()),
            autogen_warning: Some(
                "/* This file is auto-generated. Do not modify manually. */".to_string(),
            ),
            includes: vec![],
            sys_includes: vec!["stdint.h".to_string(), "stdbool.h".to_string()],
            no_includes: false,
            cpp_compat: true,
            documentation: true,
            documentation_style: cbindgen::DocumentationStyle::C99,
            ..Default::default()
        };

        cbindgen::Builder::new()
            .with_crate(crate_dir)
            .with_config(config)
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(Path::new(&out_dir).join("ios_sdk_ffi.h"));
    }
}
