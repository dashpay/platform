//! Simple performance benchmarks for wasm-drive-verify
//!
//! This file contains timing benchmarks for various verification functions
//! to measure performance characteristics with different proof sizes.

use dpp::version::PlatformVersion;
use js_sys::Uint8Array;
use std::time::Instant;
use wasm_bindgen::JsValue;
use wasm_drive_verify::contract_verification::verify_contract::verify_contract as wasm_verify_contract;
use wasm_drive_verify::document_verification::verify_proof::verify_document_proof;

// Helper functions
fn create_mock_proof(size: usize) -> Uint8Array {
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    Uint8Array::from(&data[..])
}

fn create_mock_id(seed: u8) -> Uint8Array {
    let data: Vec<u8> = vec![seed; 32];
    Uint8Array::from(&data[..])
}

/// Time a function execution
fn time_function<F: Fn()>(name: &str, iterations: u32, f: F) {
    let start = Instant::now();

    for _ in 0..iterations {
        f();
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations;

    println!(
        "{}: {} iterations in {:?} (avg: {:?})",
        name, iterations, duration, avg_duration
    );
}

fn main() {
    println!("Running wasm-drive-verify benchmarks...\n");

    // Test different proof sizes
    let proof_sizes = vec![(1024, "1KB"), (10 * 1024, "10KB"), (100 * 1024, "100KB")];

    // Benchmark identity verification
    println!("=== Identity Verification ===");
    for (size, label) in &proof_sizes {
        let proof = create_mock_proof(*size);
        let identity_id = create_mock_id(1);

        time_function(
            &format!("verify_full_identity_by_identity_id ({})", label),
            100,
            || {
                use wasm_drive_verify::identity_verification::verify_full_identity_by_identity_id;
                let _ = verify_full_identity_by_identity_id(&proof, false, &identity_id, 1);
            },
        );
    }

    println!("\n=== Document Verification ===");
    for (size, label) in &proof_sizes {
        let proof = create_mock_proof(*size);
        let contract_js = JsValue::from(create_mock_proof(512));

        time_function(&format!("verify_document_proof ({})", label), 100, || {
            let where_clauses = JsValue::UNDEFINED;
            let order_by = JsValue::UNDEFINED;

            let _ = verify_document_proof(
                &proof,
                &contract_js,
                "test_doc",
                &where_clauses,
                &order_by,
                None,
                None,
                None,
                false,
                None,
                1,
            );
        });
    }

    println!("\n=== Contract Verification ===");
    for (size, label) in &proof_sizes {
        let proof = create_mock_proof(*size);
        let contract_id = create_mock_id(3);

        time_function(&format!("verify_contract ({})", label), 100, || {
            let _ = wasm_verify_contract(&proof, None, false, false, &contract_id, 1);
        });
    }

    println!("\n=== Platform Version Validation ===");
    time_function("PlatformVersion::get (all versions)", 1000, || {
        for version in 1..=9 {
            let _ = PlatformVersion::get(version);
        }
    });

    println!("\n=== Getter Performance ===");
    let data_sizes = vec![32, 256, 1024, 10240];
    for size in data_sizes {
        let data = vec![0u8; size];

        time_function(&format!("Uint8Array::from ({}B)", size), 1000, || {
            let _ = Uint8Array::from(&data[..]);
        });
    }

    println!("\nBenchmarks complete!");
}
