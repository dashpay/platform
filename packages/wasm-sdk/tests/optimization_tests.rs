//! Optimization and performance tests

use wasm_bindgen_test::*;
use wasm_sdk::optimize::{
    BatchOptimizer, CompressionUtils, FeatureFlags, MemoryOptimizer,
    PerformanceMonitor, clear_string_cache, get_optimization_recommendations,
    init_string_cache, intern_string, optimize_uint8_array
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_feature_flags() {
    // Test default feature flags
    let default_flags = FeatureFlags::new();
    let size_reduction = default_flags.get_estimated_size_reduction();
    assert!(size_reduction.contains("No size reduction"), "Default should have all features");
    
    // Test minimal feature flags
    let minimal_flags = FeatureFlags::minimal();
    let minimal_reduction = minimal_flags.get_estimated_size_reduction();
    assert!(minimal_reduction.contains("size reduction"), "Minimal should show reduction");
    
    // Test custom feature flags
    let mut custom_flags = FeatureFlags::new();
    custom_flags.set_enable_tokens(false);
    custom_flags.set_enable_withdrawals(false);
    custom_flags.set_enable_voting(false);
    
    let custom_reduction = custom_flags.get_estimated_size_reduction();
    assert!(custom_reduction.contains("tokens"), "Should mention disabled tokens");
    assert!(custom_reduction.contains("withdrawals"), "Should mention disabled withdrawals");
}

#[wasm_bindgen_test]
fn test_memory_optimizer() {
    let mut optimizer = MemoryOptimizer::new();
    
    // Track some allocations
    optimizer.track_allocation(1024);
    optimizer.track_allocation(2048);
    optimizer.track_allocation(512);
    
    let stats = optimizer.get_stats();
    assert!(stats.contains("Allocations: 3"), "Should track 3 allocations");
    assert!(stats.contains("Total size: 3584"), "Should track total size");
    
    // Reset stats
    optimizer.reset();
    let reset_stats = optimizer.get_stats();
    assert!(reset_stats.contains("Allocations: 0"), "Should reset allocations");
}

#[wasm_bindgen_test]
fn test_batch_optimizer() {
    let mut optimizer = BatchOptimizer::new();
    
    // Test default settings
    assert_eq!(optimizer.get_optimal_batch_count(100), 10, "Should calculate batch count");
    
    // Test custom batch size
    optimizer.set_batch_size(20);
    assert_eq!(optimizer.get_optimal_batch_count(100), 5, "Should use custom batch size");
    
    // Test batch boundaries
    let boundaries = optimizer.get_batch_boundaries(100, 2);
    let start = js_sys::Reflect::get(&boundaries, &"start".into()).unwrap();
    let end = js_sys::Reflect::get(&boundaries, &"end".into()).unwrap();
    let size = js_sys::Reflect::get(&boundaries, &"size".into()).unwrap();
    
    assert_eq!(start.as_f64().unwrap() as usize, 40);
    assert_eq!(end.as_f64().unwrap() as usize, 60);
    assert_eq!(size.as_f64().unwrap() as usize, 20);
    
    // Test max concurrent setting
    optimizer.set_max_concurrent(5);
    // This is just a setter, verify it doesn't crash
}

#[wasm_bindgen_test]
fn test_string_interning() {
    init_string_cache();
    
    // Intern some strings
    let s1 = intern_string("test_string");
    let s2 = intern_string("test_string");
    let s3 = intern_string("different_string");
    
    // Same strings should be equal
    assert_eq!(s1, s2, "Interned strings should be equal");
    assert_ne!(s1, s3, "Different strings should not be equal");
    
    // Clear cache
    clear_string_cache();
    // After clearing, new interns should work
    let s4 = intern_string("test_string");
    assert_eq!(s4, "test_string");
}

#[wasm_bindgen_test]
fn test_compression_utils() {
    // Test should compress logic
    assert!(!CompressionUtils::should_compress(100), "Small data shouldn't compress");
    assert!(!CompressionUtils::should_compress(1000), "1KB shouldn't compress");
    assert!(CompressionUtils::should_compress(2000), "2KB should compress");
    
    // Test compression ratio estimation
    let uniform_data = vec![42u8; 1000];
    let ratio1 = CompressionUtils::estimate_compression_ratio(&uniform_data);
    assert!(ratio1 < 0.5, "Uniform data should have low compression ratio");
    
    let random_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let ratio2 = CompressionUtils::estimate_compression_ratio(&random_data);
    assert!(ratio2 > ratio1, "Random data should have higher compression ratio");
}

#[wasm_bindgen_test]
fn test_performance_monitor() {
    let mut monitor = PerformanceMonitor::new();
    
    // Mark some performance points
    monitor.mark("start");
    
    // Simulate some work with a small delay
    let start = js_sys::Date::now();
    while js_sys::Date::now() - start < 10.0 {}
    
    monitor.mark("after_work");
    
    // Get report
    let report = monitor.get_report();
    assert!(report.contains("Performance Report"), "Should have report header");
    assert!(report.contains("start"), "Should contain start mark");
    assert!(report.contains("after_work"), "Should contain after_work mark");
    assert!(report.contains("delta:"), "Should show delta times");
    
    // Reset monitor
    monitor.reset();
    monitor.mark("new_start");
    let new_report = monitor.get_report();
    assert!(!new_report.contains("after_work"), "Should not contain old marks");
    assert!(new_report.contains("new_start"), "Should contain new mark");
}

#[wasm_bindgen_test]
fn test_uint8_array_optimization() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let optimized = optimize_uint8_array(&data);
    
    // Verify the array contains the same data
    assert_eq!(optimized.length(), 8);
    for i in 0..8 {
        assert_eq!(optimized.get_index(i), data[i as usize]);
    }
}

#[wasm_bindgen_test]
fn test_optimization_recommendations() {
    let recommendations = get_optimization_recommendations();
    
    assert!(recommendations.length() > 0, "Should have recommendations");
    
    // Check for some expected recommendations
    let has_feature_flags = (0..recommendations.length()).any(|i| {
        recommendations.get(i).as_string()
            .map(|s| s.contains("FeatureFlags"))
            .unwrap_or(false)
    });
    assert!(has_feature_flags, "Should recommend using FeatureFlags");
    
    let has_caching = (0..recommendations.length()).any(|i| {
        recommendations.get(i).as_string()
            .map(|s| s.contains("caching"))
            .unwrap_or(false)
    });
    assert!(has_caching, "Should recommend caching");
}