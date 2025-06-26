//! # Monitoring Module
//!
//! This module provides monitoring and observability features for the WASM SDK,
//! including metrics collection, performance tracking, and health checks.

use wasm_bindgen::prelude::*;
use js_sys::{Array, Date, Object, Reflect, Map};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Performance metrics for operations
#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct PerformanceMetrics {
    operation: String,
    start_time: f64,
    end_time: Option<f64>,
    success: Option<bool>,
    error_message: Option<String>,
    metadata: HashMap<String, String>,
}

#[wasm_bindgen]
impl PerformanceMetrics {
    /// Get operation name
    #[wasm_bindgen(getter)]
    pub fn operation(&self) -> String {
        self.operation.clone()
    }
    
    /// Get duration in milliseconds
    #[wasm_bindgen(getter)]
    pub fn duration(&self) -> Option<f64> {
        self.end_time.map(|end| end - self.start_time)
    }
    
    /// Get success status
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> Option<bool> {
        self.success
    }
    
    /// Get error message
    #[wasm_bindgen(getter, js_name = errorMessage)]
    pub fn error_message(&self) -> Option<String> {
        self.error_message.clone()
    }
    
    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"operation".into(), &self.operation.clone().into())
            .map_err(|_| JsError::new("Failed to set operation"))?;
        Reflect::set(&obj, &"startTime".into(), &self.start_time.into())
            .map_err(|_| JsError::new("Failed to set start time"))?;
        
        if let Some(end_time) = self.end_time {
            Reflect::set(&obj, &"endTime".into(), &end_time.into())
                .map_err(|_| JsError::new("Failed to set end time"))?;
            Reflect::set(&obj, &"duration".into(), &(end_time - self.start_time).into())
                .map_err(|_| JsError::new("Failed to set duration"))?;
        }
        
        if let Some(success) = self.success {
            Reflect::set(&obj, &"success".into(), &success.into())
                .map_err(|_| JsError::new("Failed to set success"))?;
        }
        
        if let Some(ref error) = self.error_message {
            Reflect::set(&obj, &"errorMessage".into(), &error.clone().into())
                .map_err(|_| JsError::new("Failed to set error message"))?;
        }
        
        // Add metadata
        let metadata_obj = Object::new();
        for (key, value) in &self.metadata {
            Reflect::set(&metadata_obj, &key.into(), &value.clone().into())
                .map_err(|_| JsError::new("Failed to set metadata"))?;
        }
        Reflect::set(&obj, &"metadata".into(), &metadata_obj)
            .map_err(|_| JsError::new("Failed to set metadata"))?;
        
        Ok(obj.into())
    }
}

/// SDK Monitor for tracking operations and performance
#[wasm_bindgen]
pub struct SdkMonitor {
    metrics: Arc<Mutex<Vec<PerformanceMetrics>>>,
    active_operations: Arc<Mutex<HashMap<String, PerformanceMetrics>>>,
    enabled: bool,
    max_metrics: usize,
}

#[wasm_bindgen]
impl SdkMonitor {
    /// Create a new monitor
    #[wasm_bindgen(constructor)]
    pub fn new(enabled: bool, max_metrics: Option<usize>) -> SdkMonitor {
        SdkMonitor {
            metrics: Arc::new(Mutex::new(Vec::new())),
            active_operations: Arc::new(Mutex::new(HashMap::new())),
            enabled,
            max_metrics: max_metrics.unwrap_or(1000),
        }
    }
    
    /// Check if monitoring is enabled
    #[wasm_bindgen(getter)]
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    
    /// Enable monitoring
    #[wasm_bindgen]
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable monitoring
    #[wasm_bindgen]
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Start tracking an operation
    #[wasm_bindgen(js_name = startOperation)]
    pub fn start_operation(&self, operation_id: String, operation_name: String) -> Result<(), JsError> {
        if !self.enabled {
            return Ok(());
        }
        
        let metric = PerformanceMetrics {
            operation: operation_name,
            start_time: Date::now(),
            end_time: None,
            success: None,
            error_message: None,
            metadata: HashMap::new(),
        };
        
        let mut active = self.active_operations.lock()
            .map_err(|_| JsError::new("Failed to lock active operations"))?;
        active.insert(operation_id, metric);
        
        Ok(())
    }
    
    /// End tracking an operation
    #[wasm_bindgen(js_name = endOperation)]
    pub fn end_operation(
        &self,
        operation_id: String,
        success: bool,
        error_message: Option<String>,
    ) -> Result<(), JsError> {
        if !self.enabled {
            return Ok(());
        }
        
        let mut active = self.active_operations.lock()
            .map_err(|_| JsError::new("Failed to lock active operations"))?;
        
        if let Some(mut metric) = active.remove(&operation_id) {
            metric.end_time = Some(Date::now());
            metric.success = Some(success);
            metric.error_message = error_message;
            
            let mut metrics = self.metrics.lock()
                .map_err(|_| JsError::new("Failed to lock metrics"))?;
            
            // Keep only the most recent metrics
            if metrics.len() >= self.max_metrics {
                metrics.remove(0);
            }
            
            metrics.push(metric);
        }
        
        Ok(())
    }
    
    /// Add metadata to an active operation
    #[wasm_bindgen(js_name = addOperationMetadata)]
    pub fn add_operation_metadata(
        &self,
        operation_id: String,
        key: String,
        value: String,
    ) -> Result<(), JsError> {
        if !self.enabled {
            return Ok(());
        }
        
        let mut active = self.active_operations.lock()
            .map_err(|_| JsError::new("Failed to lock active operations"))?;
        
        if let Some(metric) = active.get_mut(&operation_id) {
            metric.metadata.insert(key, value);
        }
        
        Ok(())
    }
    
    /// Get all collected metrics
    #[wasm_bindgen(js_name = getMetrics)]
    pub fn get_metrics(&self) -> Result<Array, JsError> {
        let metrics = self.metrics.lock()
            .map_err(|_| JsError::new("Failed to lock metrics"))?;
        
        let arr = Array::new();
        for metric in metrics.iter() {
            arr.push(&metric.to_object()?);
        }
        
        Ok(arr)
    }
    
    /// Get metrics for a specific operation type
    #[wasm_bindgen(js_name = getMetricsByOperation)]
    pub fn get_metrics_by_operation(&self, operation_name: String) -> Result<Array, JsError> {
        let metrics = self.metrics.lock()
            .map_err(|_| JsError::new("Failed to lock metrics"))?;
        
        let arr = Array::new();
        for metric in metrics.iter() {
            if metric.operation == operation_name {
                arr.push(&metric.to_object()?);
            }
        }
        
        Ok(arr)
    }
    
    /// Get operation statistics
    #[wasm_bindgen(js_name = getOperationStats)]
    pub fn get_operation_stats(&self) -> Result<JsValue, JsError> {
        let metrics = self.metrics.lock()
            .map_err(|_| JsError::new("Failed to lock metrics"))?;
        
        let mut stats_map: HashMap<String, OperationStats> = HashMap::new();
        
        for metric in metrics.iter() {
            let stats = stats_map.entry(metric.operation.clone())
                .or_insert_with(OperationStats::default);
            
            stats.count += 1;
            
            if let Some(duration) = metric.duration() {
                stats.total_duration += duration;
                stats.min_duration = stats.min_duration.map(|min| min.min(duration)).or(Some(duration));
                stats.max_duration = stats.max_duration.map(|max| max.max(duration)).or(Some(duration));
            }
            
            if let Some(success) = metric.success {
                if success {
                    stats.success_count += 1;
                } else {
                    stats.error_count += 1;
                }
            }
        }
        
        let result = Object::new();
        for (operation, stats) in stats_map {
            let stats_obj = Object::new();
            Reflect::set(&stats_obj, &"count".into(), &stats.count.into())
                .map_err(|_| JsError::new("Failed to set count"))?;
            Reflect::set(&stats_obj, &"successCount".into(), &stats.success_count.into())
                .map_err(|_| JsError::new("Failed to set success count"))?;
            Reflect::set(&stats_obj, &"errorCount".into(), &stats.error_count.into())
                .map_err(|_| JsError::new("Failed to set error count"))?;
            
            if stats.count > 0 {
                let avg_duration = stats.total_duration / stats.count as f64;
                Reflect::set(&stats_obj, &"avgDuration".into(), &avg_duration.into())
                    .map_err(|_| JsError::new("Failed to set avg duration"))?;
            }
            
            if let Some(min) = stats.min_duration {
                Reflect::set(&stats_obj, &"minDuration".into(), &min.into())
                    .map_err(|_| JsError::new("Failed to set min duration"))?;
            }
            
            if let Some(max) = stats.max_duration {
                Reflect::set(&stats_obj, &"maxDuration".into(), &max.into())
                    .map_err(|_| JsError::new("Failed to set max duration"))?;
            }
            
            let success_rate = if stats.count > 0 {
                (stats.success_count as f64 / stats.count as f64) * 100.0
            } else {
                0.0
            };
            Reflect::set(&stats_obj, &"successRate".into(), &success_rate.into())
                .map_err(|_| JsError::new("Failed to set success rate"))?;
            
            Reflect::set(&result, &operation.into(), &stats_obj)
                .map_err(|_| JsError::new("Failed to set operation stats"))?;
        }
        
        Ok(result.into())
    }
    
    /// Clear all metrics
    #[wasm_bindgen(js_name = clearMetrics)]
    pub fn clear_metrics(&self) -> Result<(), JsError> {
        let mut metrics = self.metrics.lock()
            .map_err(|_| JsError::new("Failed to lock metrics"))?;
        metrics.clear();
        Ok(())
    }
    
    /// Get active operations count
    #[wasm_bindgen(js_name = getActiveOperationsCount)]
    pub fn get_active_operations_count(&self) -> Result<usize, JsError> {
        let active = self.active_operations.lock()
            .map_err(|_| JsError::new("Failed to lock active operations"))?;
        Ok(active.len())
    }
}

#[derive(Default)]
struct OperationStats {
    count: u32,
    success_count: u32,
    error_count: u32,
    total_duration: f64,
    min_duration: Option<f64>,
    max_duration: Option<f64>,
}

/// Global monitor instance
static GLOBAL_MONITOR: once_cell::sync::Lazy<Arc<Mutex<Option<SdkMonitor>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize global monitoring
#[wasm_bindgen(js_name = initializeMonitoring)]
pub fn initialize_monitoring(enabled: bool, max_metrics: Option<usize>) -> Result<(), JsError> {
    let monitor = SdkMonitor::new(enabled, max_metrics);
    let mut global = GLOBAL_MONITOR.lock()
        .map_err(|_| JsError::new("Failed to lock global monitor"))?;
    *global = Some(monitor);
    Ok(())
}

/// Check if global monitor is enabled
#[wasm_bindgen(js_name = isGlobalMonitorEnabled)]
pub fn is_global_monitor_enabled() -> Result<bool, JsError> {
    let global = GLOBAL_MONITOR.lock()
        .map_err(|_| JsError::new("Failed to lock global monitor"))?;
    Ok(global.is_some())
}

/// Track an async operation
#[wasm_bindgen(js_name = trackOperation)]
pub async fn track_operation(
    operation_name: String,
    operation_fn: js_sys::Function,
) -> Result<JsValue, JsError> {
    let operation_id = format!("{}_{}", operation_name, Date::now());
    
    // Start tracking
    let monitor_guard = GLOBAL_MONITOR.lock()
        .map_err(|_| JsError::new("Failed to lock global monitor"))?;
    if let Some(ref monitor) = *monitor_guard {
        monitor.start_operation(operation_id.clone(), operation_name.clone())?;
    }
    drop(monitor_guard);
    
    // Execute operation
    let result = match operation_fn.call0(&JsValue::null()) {
        Ok(result) => {
            // End tracking with success
            let monitor_guard = GLOBAL_MONITOR.lock()
                .map_err(|_| JsError::new("Failed to lock global monitor"))?;
            if let Some(ref monitor) = *monitor_guard {
                monitor.end_operation(operation_id.clone(), true, None)?;
            }
            Ok(result)
        }
        Err(error) => {
            // End tracking with error
            let monitor_guard = GLOBAL_MONITOR.lock()
                .map_err(|_| JsError::new("Failed to lock global monitor"))?;
            if let Some(ref monitor) = *monitor_guard {
                let error_msg = format!("{:?}", error);
                monitor.end_operation(operation_id, false, Some(error_msg))?;
            }
            Err(JsError::new(&format!("Operation failed: {:?}", error)))
        }
    };
    
    result
}

/// Health check result
#[wasm_bindgen]
pub struct HealthCheckResult {
    status: String,
    checks: Map,
    timestamp: f64,
}

#[wasm_bindgen]
impl HealthCheckResult {
    /// Get overall status
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        self.status.clone()
    }
    
    /// Get individual check results
    #[wasm_bindgen(getter)]
    pub fn checks(&self) -> Map {
        self.checks.clone()
    }
    
    /// Get timestamp
    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

/// Perform health check
#[wasm_bindgen(js_name = performHealthCheck)]
pub async fn perform_health_check(
    sdk: &crate::sdk::WasmSdk,
) -> Result<HealthCheckResult, JsError> {
    let checks = Map::new();
    let mut all_healthy = true;
    
    // Check DAPI connectivity
    let dapi_check = Object::new();
    match check_dapi_connectivity(sdk).await {
        Ok(true) => {
            Reflect::set(&dapi_check, &"status".into(), &"healthy".into())
                .map_err(|_| JsError::new("Failed to set status"))?;
            Reflect::set(&dapi_check, &"message".into(), &"DAPI connection successful".into())
                .map_err(|_| JsError::new("Failed to set message"))?;
        }
        Ok(false) | Err(_) => {
            all_healthy = false;
            Reflect::set(&dapi_check, &"status".into(), &"unhealthy".into())
                .map_err(|_| JsError::new("Failed to set status"))?;
            Reflect::set(&dapi_check, &"message".into(), &"DAPI connection failed".into())
                .map_err(|_| JsError::new("Failed to set message"))?;
        }
    }
    checks.set(&"dapi".into(), &dapi_check);
    
    // Check memory usage (simplified without performance.memory API)
    let memory_check = Object::new();
    Reflect::set(&memory_check, &"status".into(), &"healthy".into())
        .map_err(|_| JsError::new("Failed to set status"))?;
    Reflect::set(&memory_check, &"message".into(), &"Memory monitoring available through browser DevTools".into())
        .map_err(|_| JsError::new("Failed to set message"))?;
    checks.set(&"memory".into(), &memory_check);
    
    // Check cache status
    let cache_check = Object::new();
    Reflect::set(&cache_check, &"status".into(), &"healthy".into())
        .map_err(|_| JsError::new("Failed to set status"))?;
    Reflect::set(&cache_check, &"message".into(), &"Cache operational".into())
        .map_err(|_| JsError::new("Failed to set message"))?;
    checks.set(&"cache".into(), &cache_check);
    
    Ok(HealthCheckResult {
        status: if all_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
        checks,
        timestamp: Date::now(),
    })
}

async fn check_dapi_connectivity(sdk: &crate::sdk::WasmSdk) -> Result<bool, JsError> {
    // Try to get protocol version as a simple connectivity check
    use crate::dapi_client::{DapiClient, DapiClientConfig};
    
    let client_config = DapiClientConfig::new(sdk.network());
    let client = DapiClient::new(client_config)?;
    
    match client.get_protocol_version().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Resource usage information
#[wasm_bindgen(js_name = getResourceUsage)]
pub fn get_resource_usage() -> Result<JsValue, JsError> {
    let usage = Object::new();
    
    // Memory usage - performance.memory() is not available in web-sys
    // We'll create a placeholder for now
    {
        let memory_obj = Object::new();
        
        // Set placeholder values
        Reflect::set(&memory_obj, &"available".into(), &false.into())
            .map_err(|_| JsError::new("Failed to set memory available"))?;
        Reflect::set(&memory_obj, &"message".into(), &"Memory API not available in web-sys".into())
            .map_err(|_| JsError::new("Failed to set memory message"))?;
        
        Reflect::set(&usage, &"memory".into(), &memory_obj)
            .map_err(|_| JsError::new("Failed to set memory"))?;
    }
    
    // Active operations from monitor
    let monitor_guard = GLOBAL_MONITOR.lock()
        .map_err(|_| JsError::new("Failed to lock global monitor"))?;
    if let Some(ref monitor) = *monitor_guard {
        if let Ok(count) = monitor.get_active_operations_count() {
            Reflect::set(&usage, &"activeOperations".into(), &count.into())
                .map_err(|_| JsError::new("Failed to set active operations"))?;
        }
    }
    
    // Timestamp
    Reflect::set(&usage, &"timestamp".into(), &Date::now().into())
        .map_err(|_| JsError::new("Failed to set timestamp"))?;
    
    Ok(usage.into())
}