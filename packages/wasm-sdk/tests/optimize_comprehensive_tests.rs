//! Comprehensive optimization module tests

use wasm_bindgen_test::*;
use wasm_sdk::{
    optimize::{
        FeatureFlags, MemoryOptimizer, BatchOptimizer, CompressionUtils, 
        PerformanceMonitor, optimize_uint8_array, intern_string, 
        init_string_cache, clear_string_cache, get_optimization_recommendations
    },
    start,
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_feature_flags_default() {
    start().await.expect("Failed to start WASM");
    
    let flags = FeatureFlags::new();
    
    // Most features should be enabled by default
    let size_reduction = flags.get_estimated_size_reduction();
    assert!(size_reduction.contains("voting")); // Voting is disabled by default
}

#[wasm_bindgen_test]
async fn test_feature_flags_minimal() {
    start().await.expect("Failed to start WASM");
    
    let flags = FeatureFlags::minimal();
    
    let size_reduction = flags.get_estimated_size_reduction();
    assert!(size_reduction.contains("tokens"));
    assert!(size_reduction.contains("withdrawals"));
    assert!(size_reduction.contains("voting"));
    assert!(size_reduction.contains("cache"));
    assert!(size_reduction.contains("proof verification"));
}

#[wasm_bindgen_test]
async fn test_feature_flags_custom() {
    start().await.expect("Failed to start WASM");
    
    let mut flags = FeatureFlags::new();
    
    flags.set_enable_tokens(false);
    flags.set_enable_withdrawals(false);
    flags.set_enable_cache(false);
    
    let size_reduction = flags.get_estimated_size_reduction();
    assert!(size_reduction.contains("~105KB")); // 50 + 30 + 25
}

#[wasm_bindgen_test]
async fn test_memory_optimizer() {
    start().await.expect("Failed to start WASM");
    
    let mut optimizer = MemoryOptimizer::new();
    
    // Track some allocations
    optimizer.track_allocation(1024);
    optimizer.track_allocation(2048);
    optimizer.track_allocation(512);
    
    let stats = optimizer.get_stats();
    assert!(stats.contains("Allocations: 3"));
    assert!(stats.contains("Total size: 3584 bytes"));
    
    // Reset and check
    optimizer.reset();
    let stats = optimizer.get_stats();
    assert!(stats.contains("Allocations: 0"));
    assert!(stats.contains("Total size: 0 bytes"));
}

#[wasm_bindgen_test]
async fn test_batch_optimizer() {
    start().await.expect("Failed to start WASM");
    
    let mut optimizer = BatchOptimizer::new();
    
    // Test default values
    assert_eq!(optimizer.get_optimal_batch_count(100), 10); // 100/10 = 10
    
    // Set custom batch size
    optimizer.set_batch_size(25);
    assert_eq!(optimizer.get_optimal_batch_count(100), 4); // 100/25 = 4
    
    // Test batch boundaries
    let boundaries = optimizer.get_batch_boundaries(100, 0);
    assert_eq!(
        js_sys::Reflect::get(&boundaries, &"start".into()).unwrap().as_f64().unwrap(),
        0.0
    );
    assert_eq!(
        js_sys::Reflect::get(&boundaries, &"end".into()).unwrap().as_f64().unwrap(),
        25.0
    );
    assert_eq!(
        js_sys::Reflect::get(&boundaries, &"size".into()).unwrap().as_f64().unwrap(),
        25.0
    );
    
    // Test last batch
    let last_batch = optimizer.get_batch_boundaries(100, 3);
    assert_eq!(
        js_sys::Reflect::get(&last_batch, &"start".into()).unwrap().as_f64().unwrap(),
        75.0
    );
    assert_eq!(
        js_sys::Reflect::get(&last_batch, &"end".into()).unwrap().as_f64().unwrap(),
        100.0
    );
}

#[wasm_bindgen_test]
async fn test_batch_optimizer_limits() {
    start().await.expect("Failed to start WASM");
    
    let mut optimizer = BatchOptimizer::new();
    
    // Test size limits
    optimizer.set_batch_size(0); // Should be clamped to 1
    assert_eq!(optimizer.get_optimal_batch_count(10), 10);
    
    optimizer.set_batch_size(200); // Should be clamped to 100
    assert_eq!(optimizer.get_optimal_batch_count(200), 2);
    
    // Test concurrent limits
    optimizer.set_max_concurrent(0); // Should be clamped to 1
    optimizer.set_max_concurrent(20); // Should be clamped to 10
}

#[wasm_bindgen_test]
async fn test_optimize_uint8_array() {
    start().await.expect("Failed to start WASM");
    
    let data = vec![1, 2, 3, 4, 5];
    let array = optimize_uint8_array(&data);
    
    assert_eq!(array.length(), 5);
    assert_eq!(array.get_index(0), 1);
    assert_eq!(array.get_index(4), 5);
}

#[wasm_bindgen_test]
async fn test_string_interning() {
    start().await.expect("Failed to start WASM");
    
    init_string_cache();
    
    // Intern the same string multiple times
    let s1 = intern_string("hello world");
    let s2 = intern_string("hello world");
    let s3 = intern_string("different string");
    
    assert_eq!(s1, "hello world");
    assert_eq!(s2, "hello world");
    assert_eq!(s3, "different string");
    
    // Clear cache
    clear_string_cache();
}

#[wasm_bindgen_test]
async fn test_compression_utils() {
    start().await.expect("Failed to start WASM");
    
    // Test should compress logic
    assert!(!CompressionUtils::should_compress(100)); // Too small
    assert!(!CompressionUtils::should_compress(1000)); // Still too small
    assert!(CompressionUtils::should_compress(2000)); // Should compress
    
    // Test compression ratio estimation
    let uniform_data = vec![42u8; 1000]; // Very compressible
    let ratio1 = CompressionUtils::estimate_compression_ratio(&uniform_data);
    assert!(ratio1 < 0.5); // Should estimate good compression
    
    let random_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let ratio2 = CompressionUtils::estimate_compression_ratio(&random_data);
    assert!(ratio2 > ratio1); // Random data compresses worse
}

#[wasm_bindgen_test]
async fn test_performance_monitor() {
    start().await.expect("Failed to start WASM");
    
    let mut monitor = PerformanceMonitor::new();
    
    // Add some marks
    monitor.mark("start");
    
    // Small delay
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                10,
            )
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    
    monitor.mark("after_delay");
    monitor.mark("end");
    
    let report = monitor.get_report();
    assert!(report.contains("Performance Report:"));
    assert!(report.contains("start"));
    assert!(report.contains("after_delay"));
    assert!(report.contains("end"));
    assert!(report.contains("Total time:"));
    
    // Reset and check
    monitor.reset();
    monitor.mark("new_start");
    let new_report = monitor.get_report();
    assert!(!new_report.contains("after_delay"));
    assert!(new_report.contains("new_start"));
}

#[wasm_bindgen_test]
async fn test_optimization_recommendations() {
    start().await.expect("Failed to start WASM");
    
    let recommendations = get_optimization_recommendations();
    
    assert!(recommendations.length() > 0);
    
    // Check some expected recommendations
    let recommendations_str: Vec<String> = (0..recommendations.length())
        .map(|i| recommendations.get(i).as_string().unwrap())
        .collect();
    
    assert!(recommendations_str.iter().any(|r| r.contains("FeatureFlags")));
    assert!(recommendations_str.iter().any(|r| r.contains("compression")));
    assert!(recommendations_str.iter().any(|r| r.contains("batch")));
    assert!(recommendations_str.iter().any(|r| r.contains("caching")));
}

#[wasm_bindgen_test]
async fn test_memory_optimizer_force_gc() {
    start().await.expect("Failed to start WASM");
    
    // This just tests that force_gc doesn't crash
    MemoryOptimizer::force_gc();
}

#[wasm_bindgen_test]
async fn test_feature_flags_all_disabled() {
    start().await.expect("Failed to start WASM");
    
    let mut flags = FeatureFlags::new();
    
    // Disable everything
    flags.set_enable_identity(false);
    flags.set_enable_contracts(false);
    flags.set_enable_documents(false);
    flags.set_enable_tokens(false);
    flags.set_enable_withdrawals(false);
    flags.set_enable_voting(false);
    flags.set_enable_cache(false);
    flags.set_enable_proof_verification(false);
    
    let size_reduction = flags.get_estimated_size_reduction();
    assert!(size_reduction.contains("~225KB")); // Sum of all reductions
}

#[wasm_bindgen_test]
async fn test_compression_edge_cases() {
    start().await.expect("Failed to start WASM");
    
    // Empty data
    let empty_data: Vec<u8> = vec![];
    let ratio = CompressionUtils::estimate_compression_ratio(&empty_data);
    assert!(ratio >= 0.1 && ratio <= 1.0);
    
    // Single byte
    let single_byte = vec![42];
    let ratio = CompressionUtils::estimate_compression_ratio(&single_byte);
    assert!(ratio >= 0.1 && ratio <= 1.0);
}