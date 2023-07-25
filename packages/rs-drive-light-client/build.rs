/// Generate [uniffi] scaffolding code
fn main() {
    #[cfg(feature = "uniffi")]
    build_uniffi()
}

#[cfg(feature = "uniffi")]
fn build_uniffi() {
    let files = vec!["src/dash_drive_v0.udl"];
    for file in files {
        uniffi::generate_scaffolding(file).expect("failed to compile udf");
    }
}
