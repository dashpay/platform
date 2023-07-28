use drive_proof_verifier::uniffi_bindings::bindgen::generate_uniffi_bindings;

fn main() {
    let destination = env!("CARGO_MANIFEST_DIR").to_string() + "/bindings";

    generate_uniffi_bindings(Some(&destination))
}
