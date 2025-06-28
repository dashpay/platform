//! Unit tests for monitoring functionality

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_sdk::monitoring::*;
use wasm_sdk::sdk::WasmSdk;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_sdk_monitor_creation() {
    let monitor = SdkMonitor::new(true, Some(100));
    assert!(monitor.enabled());

    let monitor_disabled = SdkMonitor::new(false, None);
    assert!(!monitor_disabled.enabled());
}

#[wasm_bindgen_test]
fn test_monitor_enable_disable() {
    let mut monitor = SdkMonitor::new(false, None);
    assert!(!monitor.enabled());

    monitor.enable();
    assert!(monitor.enabled());

    monitor.disable();
    assert!(!monitor.enabled());
}

#[wasm_bindgen_test]
async fn test_operation_tracking() {
    let monitor = SdkMonitor::new(true, None);

    // Start an operation
    monitor
        .start_operation("test_op_1".to_string(), "TestOperation".to_string())
        .expect("Should start operation");

    // Check active operations count
    let active_count = monitor
        .get_active_operations_count()
        .expect("Should get active operations count");
    assert_eq!(active_count, 1);

    // End the operation
    monitor
        .end_operation("test_op_1".to_string(), true, None)
        .expect("Should end operation");

    // Check metrics were recorded
    let metrics = monitor.get_metrics().expect("Should get metrics");
    assert_eq!(metrics.length(), 1);

    // Verify active operations cleared
    let active_count_after = monitor
        .get_active_operations_count()
        .expect("Should get active operations count");
    assert_eq!(active_count_after, 0);
}

#[wasm_bindgen_test]
async fn test_operation_with_error() {
    let monitor = SdkMonitor::new(true, None);

    monitor
        .start_operation("error_op".to_string(), "ErrorOperation".to_string())
        .expect("Should start operation");

    monitor
        .end_operation(
            "error_op".to_string(),
            false,
            Some("Test error message".to_string()),
        )
        .expect("Should end operation with error");

    let metrics = monitor.get_metrics().expect("Should get metrics");
    assert_eq!(metrics.length(), 1);

    // Check the error was recorded
    let metric = metrics.get(0);
    let metric_obj = metric.dyn_ref::<Object>().expect("Should be an object");

    let success = Reflect::get(metric_obj, &"success".into()).expect("Should have success field");
    assert_eq!(success.as_bool(), Some(false));

    let error_msg =
        Reflect::get(metric_obj, &"errorMessage".into()).expect("Should have error message");
    assert_eq!(
        error_msg.as_string(),
        Some("Test error message".to_string())
    );
}

#[wasm_bindgen_test]
fn test_operation_metadata() {
    let monitor = SdkMonitor::new(true, None);

    monitor
        .start_operation("metadata_op".to_string(), "MetadataOperation".to_string())
        .expect("Should start operation");

    // Add metadata
    monitor
        .add_operation_metadata(
            "metadata_op".to_string(),
            "key1".to_string(),
            "value1".to_string(),
        )
        .expect("Should add metadata");

    monitor
        .add_operation_metadata(
            "metadata_op".to_string(),
            "key2".to_string(),
            "value2".to_string(),
        )
        .expect("Should add metadata");

    monitor
        .end_operation("metadata_op".to_string(), true, None)
        .expect("Should end operation");

    let metrics = monitor.get_metrics().expect("Should get metrics");
    let metric = metrics.get(0);
    let metric_obj = metric.dyn_ref::<Object>().expect("Should be an object");

    let metadata = Reflect::get(metric_obj, &"metadata".into()).expect("Should have metadata");
    let metadata_obj = metadata
        .dyn_ref::<Object>()
        .expect("Metadata should be an object");

    let value1 = Reflect::get(metadata_obj, &"key1".into()).expect("Should have key1");
    assert_eq!(value1.as_string(), Some("value1".to_string()));
}

#[wasm_bindgen_test]
fn test_metrics_by_operation() {
    let monitor = SdkMonitor::new(true, None);

    // Add multiple operations of different types
    for i in 0..3 {
        let op_id = format!("fetch_{}", i);
        monitor
            .start_operation(op_id.clone(), "FetchOperation".to_string())
            .expect("Should start operation");
        monitor
            .end_operation(op_id, true, None)
            .expect("Should end operation");
    }

    for i in 0..2 {
        let op_id = format!("broadcast_{}", i);
        monitor
            .start_operation(op_id.clone(), "BroadcastOperation".to_string())
            .expect("Should start operation");
        monitor
            .end_operation(op_id, true, None)
            .expect("Should end operation");
    }

    // Get metrics for specific operation type
    let fetch_metrics = monitor
        .get_metrics_by_operation("FetchOperation".to_string())
        .expect("Should get fetch metrics");
    assert_eq!(fetch_metrics.length(), 3);

    let broadcast_metrics = monitor
        .get_metrics_by_operation("BroadcastOperation".to_string())
        .expect("Should get broadcast metrics");
    assert_eq!(broadcast_metrics.length(), 2);
}

#[wasm_bindgen_test]
fn test_operation_statistics() {
    let monitor = SdkMonitor::new(true, None);

    // Create operations with different outcomes
    for i in 0..5 {
        let op_id = format!("test_{}", i);
        monitor
            .start_operation(op_id.clone(), "TestOp".to_string())
            .expect("Should start operation");

        // Make some operations fail
        let success = i % 2 == 0;
        let error = if success {
            None
        } else {
            Some("Error".to_string())
        };

        monitor
            .end_operation(op_id, success, error)
            .expect("Should end operation");
    }

    let stats = monitor
        .get_operation_stats()
        .expect("Should get operation stats");

    let stats_obj = stats
        .dyn_ref::<Object>()
        .expect("Stats should be an object");

    let test_op_stats =
        Reflect::get(stats_obj, &"TestOp".into()).expect("Should have TestOp stats");
    let test_op_obj = test_op_stats
        .dyn_ref::<Object>()
        .expect("TestOp stats should be an object");

    let count = Reflect::get(test_op_obj, &"count".into()).expect("Should have count");
    assert_eq!(count.as_f64(), Some(5.0));

    let success_count =
        Reflect::get(test_op_obj, &"successCount".into()).expect("Should have success count");
    assert_eq!(success_count.as_f64(), Some(3.0));

    let error_count =
        Reflect::get(test_op_obj, &"errorCount".into()).expect("Should have error count");
    assert_eq!(error_count.as_f64(), Some(2.0));

    let success_rate =
        Reflect::get(test_op_obj, &"successRate".into()).expect("Should have success rate");
    assert_eq!(success_rate.as_f64(), Some(60.0));
}

#[wasm_bindgen_test]
fn test_max_metrics_limit() {
    let monitor = SdkMonitor::new(true, Some(3));

    // Add more operations than the limit
    for i in 0..5 {
        let op_id = format!("op_{}", i);
        monitor
            .start_operation(op_id.clone(), "TestOp".to_string())
            .expect("Should start operation");
        monitor
            .end_operation(op_id, true, None)
            .expect("Should end operation");
    }

    // Should only keep the most recent 3
    let metrics = monitor.get_metrics().expect("Should get metrics");
    assert_eq!(metrics.length(), 3);
}

#[wasm_bindgen_test]
fn test_clear_metrics() {
    let monitor = SdkMonitor::new(true, None);

    // Add some operations
    for i in 0..3 {
        let op_id = format!("op_{}", i);
        monitor
            .start_operation(op_id.clone(), "TestOp".to_string())
            .expect("Should start operation");
        monitor
            .end_operation(op_id, true, None)
            .expect("Should end operation");
    }

    let metrics_before = monitor.get_metrics().expect("Should get metrics");
    assert!(metrics_before.length() > 0);

    // Clear metrics
    monitor.clear_metrics().expect("Should clear metrics");

    let metrics_after = monitor.get_metrics().expect("Should get metrics");
    assert_eq!(metrics_after.length(), 0);
}

#[wasm_bindgen_test]
fn test_disabled_monitor() {
    let monitor = SdkMonitor::new(false, None);

    // Operations should not be tracked when disabled
    monitor
        .start_operation("op1".to_string(), "TestOp".to_string())
        .expect("Should not error when disabled");
    monitor
        .end_operation("op1".to_string(), true, None)
        .expect("Should not error when disabled");

    let metrics = monitor.get_metrics().expect("Should get empty metrics");
    assert_eq!(metrics.length(), 0);
}

#[wasm_bindgen_test]
fn test_global_monitor_initialization() {
    initialize_monitoring(true, Some(100)).expect("Should initialize global monitoring");

    let monitor = get_global_monitor()
        .expect("Should get global monitor")
        .expect("Global monitor should exist");

    assert!(monitor.enabled());
}

#[wasm_bindgen_test]
async fn test_health_check() {
    use crate::common::setup_test_sdk;

    let sdk = setup_test_sdk().await;

    let health = perform_health_check(&sdk)
        .await
        .expect("Should perform health check");

    // Check status
    let status = health.status();
    assert!(status == "healthy" || status == "unhealthy");

    // Check timestamp
    assert!(health.timestamp() > 0.0);

    // Check individual checks exist
    let checks = health.checks();
    assert!(checks.has(&"dapi".into()));
    assert!(checks.has(&"memory".into()));
    assert!(checks.has(&"cache".into()));
}

#[wasm_bindgen_test]
fn test_resource_usage() {
    let usage = get_resource_usage().expect("Should get resource usage");

    let usage_obj = usage
        .dyn_ref::<Object>()
        .expect("Usage should be an object");

    // Should have timestamp
    assert!(Reflect::has(usage_obj, &"timestamp".into()).unwrap());

    // May have memory info if available
    if Reflect::has(usage_obj, &"memory".into()).unwrap() {
        let memory = Reflect::get(usage_obj, &"memory".into()).expect("Should get memory");
        assert!(!memory.is_undefined());
    }
}

#[wasm_bindgen_test]
fn test_performance_metrics_object() {
    let monitor = SdkMonitor::new(true, None);

    monitor
        .start_operation("perf_test".to_string(), "PerfTest".to_string())
        .expect("Should start operation");

    // Small delay to ensure measurable duration
    let start = js_sys::Date::now();
    while js_sys::Date::now() - start < 10.0 {}

    monitor
        .end_operation("perf_test".to_string(), true, None)
        .expect("Should end operation");

    let metrics = monitor.get_metrics().expect("Should get metrics");
    let metric = metrics.get(0);
    let metric_obj = metric.dyn_ref::<Object>().expect("Should be an object");

    // Check all expected fields
    assert!(Reflect::has(metric_obj, &"operation".into()).unwrap());
    assert!(Reflect::has(metric_obj, &"startTime".into()).unwrap());
    assert!(Reflect::has(metric_obj, &"endTime".into()).unwrap());
    assert!(Reflect::has(metric_obj, &"duration".into()).unwrap());
    assert!(Reflect::has(metric_obj, &"success".into()).unwrap());

    // Duration should be positive
    let duration = Reflect::get(metric_obj, &"duration".into()).expect("Should have duration");
    assert!(duration.as_f64().unwrap() > 0.0);
}
