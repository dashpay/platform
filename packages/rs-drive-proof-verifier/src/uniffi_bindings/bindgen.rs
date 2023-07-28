use camino::{Utf8Path, Utf8PathBuf};
use uniffi::TargetLanguage;

/// Generate UniFFI bindings for all available languages
///
/// It is intended to be called from build.rs in external crate (eg. rs-drive-proof-verifier-bindings), in order
/// to rebuild bindings. External crate must have this crate as build-dependencies.
pub fn generate_uniffi_bindings(destination: Option<&str>) {
    let cargo_dir = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));

    let udl_file = cargo_dir.join("src/dash_drive_v0.udl");
    let lib_file = drive_library_path();
    let destination = match destination {
        None => cargo_dir.join("bindings"),
        Some(d) => d.into(),
    };

    let target_languages: Vec<uniffi::TargetLanguage> = vec![
        TargetLanguage::Kotlin,
        TargetLanguage::Swift,
        TargetLanguage::Python,
        // TargetLanguage::Ruby, // Disabled due to `error: callback interfaces not implemented`- we need callbacks
    ];
    // Remove all bindings; ignore errors
    std::fs::remove_dir_all(&destination).ok();

    for lang in target_languages {
        uniffi::generate_bindings(
            &udl_file,
            None,
            vec![lang],
            Some(&destination.join(lang.to_string())),
            Some(&lib_file),
            false,
        )
        .unwrap();
    }
}

/// Return path to the Drive library file, containing uniffi code
fn drive_library_path() -> Utf8PathBuf {
    let artifacts_dir = Utf8Path::new(env!("OUT_DIR"))
        .parent()
        .expect("cannot get parent of .../out/ dir")
        .parent()
        .expect("cannot get parrent of .../rs-drive-proof-verifier-... dir")
        .parent()
        .expect("cannot get parrent of .../build/ dir");

    artifacts_dir.join("libdrive_proof_verifier.rlib")
}
