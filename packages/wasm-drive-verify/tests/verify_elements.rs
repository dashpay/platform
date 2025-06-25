use js_sys::{Array, Uint8Array};
use wasm_bindgen_test::*;
use wasm_drive_verify::governance_verification::verify_elements::verify_elements;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_elements_not_implemented() {
    // verify_elements is not implemented due to Element type limitations
    let proof = Uint8Array::new_with_length(100);
    let path = Array::new();
    let keys = Array::new();
    let platform_version = 1;

    // This should return an error explaining the limitation
    let result = verify_elements(&proof, &path, &keys, platform_version);
    assert!(result.is_err());

    let error = result.err().unwrap();
    let error_message = error.as_string().unwrap();
    assert!(error_message.contains("not available in WASM"));
    assert!(error_message.contains("Element type limitations"));
}
