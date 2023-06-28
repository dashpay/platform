/// Generate [uniffi] scaffolding code
fn main() {
    let files = vec!["src/dash_drive_v0.udl"];
    for file in files {
        uniffi::generate_scaffolding(file).expect("failed to compile udf");
    }
}
