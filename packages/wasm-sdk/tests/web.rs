//! Test runner for browser-based WASM tests

use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);

// This file serves as the entry point for running tests in a browser environment.
// To run the tests:
// 1. Build the WASM package with tests: wasm-pack test --chrome --headless
// 2. Or run interactively: wasm-pack test --chrome
//
// The tests will be executed in a real browser environment, allowing full
// testing of Web APIs like Web Crypto, IndexedDB, and other browser-specific features.