use rs_drive_light_client::bindgen::generate_uniffi_bindings;

fn main() {
    let destination = env!("CARGO_MANIFEST_DIR").to_string() + "/bindings";

    generate_uniffi_bindings(Some(&destination))
}
