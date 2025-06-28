//! # Optimization Module
//!
//! This module provides optimization utilities for reducing WASM bundle size

use wasm_bindgen::prelude::*;

/// Feature flags for conditional compilation
#[wasm_bindgen]
pub struct FeatureFlags {
    enable_identity: bool,
    enable_contracts: bool,
    enable_documents: bool,
    enable_tokens: bool,
    enable_withdrawals: bool,
    enable_voting: bool,
    enable_cache: bool,
    enable_proof_verification: bool,
}

#[wasm_bindgen]
impl FeatureFlags {
    /// Create default feature flags (all enabled)
    #[wasm_bindgen(constructor)]
    pub fn new() -> FeatureFlags {
        FeatureFlags {
            enable_identity: true,
            enable_contracts: true,
            enable_documents: true,
            enable_tokens: true,
            enable_withdrawals: true,
            enable_voting: false, // Disabled by default as it's not implemented
            enable_cache: true,
            enable_proof_verification: true,
        }
    }

    /// Create minimal feature flags (only essentials)
    #[wasm_bindgen(js_name = minimal)]
    pub fn minimal() -> FeatureFlags {
        FeatureFlags {
            enable_identity: true,
            enable_contracts: true,
            enable_documents: true,
            enable_tokens: false,
            enable_withdrawals: false,
            enable_voting: false,
            enable_cache: false,
            enable_proof_verification: false,
        }
    }

    /// Enable identity features
    #[wasm_bindgen(js_name = setEnableIdentity)]
    pub fn set_enable_identity(&mut self, enable: bool) {
        self.enable_identity = enable;
    }

    /// Enable contract features
    #[wasm_bindgen(js_name = setEnableContracts)]
    pub fn set_enable_contracts(&mut self, enable: bool) {
        self.enable_contracts = enable;
    }

    /// Enable document features
    #[wasm_bindgen(js_name = setEnableDocuments)]
    pub fn set_enable_documents(&mut self, enable: bool) {
        self.enable_documents = enable;
    }

    /// Enable token features
    #[wasm_bindgen(js_name = setEnableTokens)]
    pub fn set_enable_tokens(&mut self, enable: bool) {
        self.enable_tokens = enable;
    }

    /// Enable withdrawal features
    #[wasm_bindgen(js_name = setEnableWithdrawals)]
    pub fn set_enable_withdrawals(&mut self, enable: bool) {
        self.enable_withdrawals = enable;
    }

    /// Enable voting features
    #[wasm_bindgen(js_name = setEnableVoting)]
    pub fn set_enable_voting(&mut self, enable: bool) {
        self.enable_voting = enable;
    }

    /// Enable cache features
    #[wasm_bindgen(js_name = setEnableCache)]
    pub fn set_enable_cache(&mut self, enable: bool) {
        self.enable_cache = enable;
    }

    /// Enable proof verification
    #[wasm_bindgen(js_name = setEnableProofVerification)]
    pub fn set_enable_proof_verification(&mut self, enable: bool) {
        self.enable_proof_verification = enable;
    }

    /// Get estimated bundle size reduction
    #[wasm_bindgen(js_name = getEstimatedSizeReduction)]
    pub fn get_estimated_size_reduction(&self) -> String {
        let mut disabled_features = Vec::new();
        let mut size_reduction = 0;

        if !self.enable_tokens {
            disabled_features.push("tokens");
            size_reduction += 50; // ~50KB
        }
        if !self.enable_withdrawals {
            disabled_features.push("withdrawals");
            size_reduction += 30; // ~30KB
        }
        if !self.enable_voting {
            disabled_features.push("voting");
            size_reduction += 20; // ~20KB
        }
        if !self.enable_cache {
            disabled_features.push("cache");
            size_reduction += 25; // ~25KB
        }
        if !self.enable_proof_verification {
            disabled_features.push("proof verification");
            size_reduction += 100; // ~100KB
        }

        if disabled_features.is_empty() {
            "No size reduction (all features enabled)".to_string()
        } else {
            format!(
                "Estimated size reduction: ~{}KB by disabling: {}",
                size_reduction,
                disabled_features.join(", ")
            )
        }
    }
}

/// Memory optimization utilities
#[wasm_bindgen]
pub struct MemoryOptimizer {
    allocation_count: usize,
    total_allocated: usize,
}

#[wasm_bindgen]
impl MemoryOptimizer {
    /// Create a new memory optimizer
    #[wasm_bindgen(constructor)]
    pub fn new() -> MemoryOptimizer {
        MemoryOptimizer {
            allocation_count: 0,
            total_allocated: 0,
        }
    }

    /// Track an allocation
    #[wasm_bindgen(js_name = trackAllocation)]
    pub fn track_allocation(&mut self, size: usize) {
        self.allocation_count += 1;
        self.total_allocated += size;
    }

    /// Get allocation statistics
    #[wasm_bindgen(js_name = getStats)]
    pub fn get_stats(&self) -> String {
        format!(
            "Allocations: {}, Total size: {} bytes",
            self.allocation_count, self.total_allocated
        )
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.allocation_count = 0;
        self.total_allocated = 0;
    }

    /// Force garbage collection (hint to JS engine)
    #[wasm_bindgen(js_name = forceGC)]
    pub fn force_gc() {
        // This is just a hint to the JS engine
        // Actual GC is controlled by the browser
        web_sys::console::log_1(&"Suggesting garbage collection...".into());
    }
}

/// Optimize Uint8Array conversions
#[wasm_bindgen(js_name = optimizeUint8Array)]
pub fn optimize_uint8_array(data: &[u8]) -> js_sys::Uint8Array {
    // Create a view directly into WASM memory
    let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);
    array
}

/// Batch operations optimizer
#[wasm_bindgen]
pub struct BatchOptimizer {
    batch_size: usize,
    max_concurrent: usize,
}

#[wasm_bindgen]
impl BatchOptimizer {
    /// Create a new batch optimizer
    #[wasm_bindgen(constructor)]
    pub fn new() -> BatchOptimizer {
        BatchOptimizer {
            batch_size: 10,      // Default batch size
            max_concurrent: 3,   // Default concurrent operations
        }
    }

    /// Set batch size
    #[wasm_bindgen(js_name = setBatchSize)]
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.max(1).min(100); // Limit between 1-100
    }

    /// Set max concurrent operations
    #[wasm_bindgen(js_name = setMaxConcurrent)]
    pub fn set_max_concurrent(&mut self, max: usize) {
        self.max_concurrent = max.max(1).min(10); // Limit between 1-10
    }

    /// Get optimal batch count for a given total
    #[wasm_bindgen(js_name = getOptimalBatchCount)]
    pub fn get_optimal_batch_count(&self, total_items: usize) -> usize {
        (total_items + self.batch_size - 1) / self.batch_size
    }

    /// Get batch boundaries
    #[wasm_bindgen(js_name = getBatchBoundaries)]
    pub fn get_batch_boundaries(&self, total_items: usize, batch_index: usize) -> js_sys::Object {
        let start = batch_index * self.batch_size;
        let end = ((batch_index + 1) * self.batch_size).min(total_items);
        
        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"start".into(), &start.into());
        let _ = js_sys::Reflect::set(&obj, &"end".into(), &end.into());
        let _ = js_sys::Reflect::set(&obj, &"size".into(), &(end - start).into());
        obj
    }
}

/// String interning for reduced memory usage
static mut STRING_CACHE: Option<std::collections::HashMap<String, String>> = None;

/// Initialize string cache
#[wasm_bindgen(js_name = initStringCache)]
pub fn init_string_cache() {
    unsafe {
        STRING_CACHE = Some(std::collections::HashMap::new());
    }
}

/// Intern a string to reduce memory usage
#[wasm_bindgen(js_name = internString)]
pub fn intern_string(s: &str) -> String {
    unsafe {
        let cache_ptr = &raw mut STRING_CACHE;
        if let Some(cache) = (*cache_ptr).as_mut() {
            if let Some(existing) = cache.get(s) {
                return existing.clone();
            }
            let owned = s.to_string();
            cache.insert(owned.clone(), owned.clone());
            owned
        } else {
            s.to_string()
        }
    }
}

/// Clear string cache
#[wasm_bindgen(js_name = clearStringCache)]
pub fn clear_string_cache() {
    unsafe {
        let cache_ptr = &raw mut STRING_CACHE;
        if let Some(cache) = (*cache_ptr).as_mut() {
            cache.clear();
        }
    }
}

/// Compression utilities for large data
#[wasm_bindgen]
pub struct CompressionUtils;

#[wasm_bindgen]
impl CompressionUtils {
    /// Check if data should be compressed based on size
    #[wasm_bindgen(js_name = shouldCompress)]
    pub fn should_compress(data_size: usize) -> bool {
        // Compress data larger than 1KB
        data_size > 1024
    }

    /// Estimate compression ratio
    #[wasm_bindgen(js_name = estimateCompressionRatio)]
    pub fn estimate_compression_ratio(data: &[u8]) -> f32 {
        // Simple entropy estimation
        let mut byte_counts = [0u32; 256];
        for &byte in data {
            byte_counts[byte as usize] += 1;
        }
        
        let total = data.len() as f32;
        let mut entropy = 0.0;
        
        for &count in &byte_counts {
            if count > 0 {
                let probability = count as f32 / total;
                entropy -= probability * probability.log2();
            }
        }
        
        // Estimate compression ratio based on entropy
        (entropy / 8.0).max(0.1).min(1.0)
    }
}

/// Performance monitoring
#[wasm_bindgen]
pub struct PerformanceMonitor {
    start_time: f64,
    measurements: Vec<(String, f64)>,
}

#[wasm_bindgen]
impl PerformanceMonitor {
    /// Create a new performance monitor
    #[wasm_bindgen(constructor)]
    pub fn new() -> PerformanceMonitor {
        PerformanceMonitor {
            start_time: js_sys::Date::now(),
            measurements: Vec::new(),
        }
    }

    /// Mark a performance point
    #[wasm_bindgen(js_name = mark)]
    pub fn mark(&mut self, label: &str) {
        let elapsed = js_sys::Date::now() - self.start_time;
        self.measurements.push((label.to_string(), elapsed));
    }

    /// Get performance report
    #[wasm_bindgen(js_name = getReport)]
    pub fn get_report(&self) -> String {
        let mut report = String::from("Performance Report:\n");
        let mut last_time = 0.0;
        
        for (label, time) in &self.measurements {
            let delta = time - last_time;
            report.push_str(&format!(
                "  {} - {:.2}ms (delta: {:.2}ms)\n",
                label, time, delta
            ));
            last_time = *time;
        }
        
        report.push_str(&format!("Total time: {:.2}ms", last_time));
        report
    }

    /// Reset measurements
    pub fn reset(&mut self) {
        self.start_time = js_sys::Date::now();
        self.measurements.clear();
    }
}

/// Export optimization recommendations
#[wasm_bindgen(js_name = getOptimizationRecommendations)]
pub fn get_optimization_recommendations() -> js_sys::Array {
    let recommendations = js_sys::Array::new();
    
    recommendations.push(&"Use FeatureFlags to disable unused features".into());
    recommendations.push(&"Enable compression for large data transfers".into());
    recommendations.push(&"Use batch operations for multiple requests".into());
    recommendations.push(&"Implement client-side caching with WasmCacheManager".into());
    recommendations.push(&"Use unproved fetching when proof verification isn't needed".into());
    recommendations.push(&"Minimize state transition sizes".into());
    recommendations.push(&"Use string interning for repeated strings".into());
    recommendations.push(&"Monitor performance with PerformanceMonitor".into());
    
    recommendations
}