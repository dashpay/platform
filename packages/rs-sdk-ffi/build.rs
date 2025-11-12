use std::env;
use std::path::Path;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    // Only generate bindings when explicitly requested
    println!(
        "cargo:warning=Build script running, GENERATE_BINDINGS={:?}",
        env::var("GENERATE_BINDINGS")
    );
    if env::var("GENERATE_BINDINGS").is_ok() {
        println!("cargo:warning=Generating unified SDK bindings with cbindgen");
        println!("cargo:warning=OUT_DIR={}", out_dir);

        // Enhanced cbindgen configuration for unified SDK
        let config = cbindgen::Config {
            language: cbindgen::Language::C,
            pragma_once: true,
            include_guard: Some("DASH_SDK_FFI_H".to_string()),
            autogen_warning: Some(
                "/* This file is auto-generated. Do not modify manually. */\n/* Unified Dash SDK - includes both Core (SPV) and Platform functionality */".to_string(),
            ),
            includes: vec![],
            sys_includes: vec!["stdint.h".to_string(), "stdbool.h".to_string()],
            no_includes: false,
            cpp_compat: true,
            documentation: true,
            documentation_style: cbindgen::DocumentationStyle::C99,
            // Enhanced export configuration from dash-unified-ffi-old
            export: cbindgen::ExportConfig {
                include: vec![
                    "dash_sdk_*".to_string(),      // Platform SDK functions
                    "dash_core_*".to_string(),     // Core SDK wrapper functions  
                    "dash_spv_*".to_string(),      // Core SDK direct functions
                    "dash_unified_*".to_string(),  // Unified SDK functions
                    "FFI*".to_string(),            // All FFI types
                    "DashSDK*".to_string(),        // Platform SDK types
                    "CoreSDK*".to_string(),        // Core SDK wrapper types
                ],
                exclude: vec![
                    "*_internal_*".to_string(),    // Exclude internal functions
                ],
                item_types: vec![
                    cbindgen::ItemType::Functions,
                    cbindgen::ItemType::Structs,
                    cbindgen::ItemType::Enums,
                    cbindgen::ItemType::Constants,
                    cbindgen::ItemType::Globals,
                    cbindgen::ItemType::Typedefs,
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        // Build unified header with dependency parsing always enabled
        let builder = cbindgen::Builder::new()
            .with_crate(&crate_dir)
            .with_parse_deps(true) // Always parse dependencies for complete type definitions
            .with_config(config);

        builder
            .generate()
            .expect("Unable to generate unified bindings")
            .write_to_file(Path::new(&out_dir).join("dash_sdk_ffi.h"));

        println!(
            "cargo:warning=Unified header generated successfully at {}/dash_sdk_ffi.h",
            out_dir
        );

        // Run header combination script to include missing Core SDK types
        let combine_script = Path::new(&crate_dir).join("combine_headers.sh");
        if combine_script.exists() {
            println!("cargo:warning=Running header combination script...");
            let output = std::process::Command::new("bash")
                .arg(&combine_script)
                .current_dir(&crate_dir)
                .output()
                .expect("Failed to run header combination script");

            if output.status.success() {
                println!("cargo:warning=Header combination completed successfully");
            } else {
                println!(
                    "cargo:warning=Header combination failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
