use crate::error::WasmSdkError;
use once_cell::sync::OnceCell;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::Registry;

static RELOAD_HANDLE: OnceCell<reload::Handle<EnvFilter, Registry>> = OnceCell::new();

fn normalize_level_or_filter(input: &str) -> Result<String, WasmSdkError> {
    let s = input.trim();
    if s.is_empty() {
        return Err(WasmSdkError::invalid_argument("Empty log level/filter"));
    }

    // Accept simple levels (case-insensitive)
    match s.to_ascii_lowercase().as_str() {
        "off" | "error" | "warn" | "info" | "debug" | "trace" => Ok(s.to_ascii_lowercase()),
        _ => {
            // Otherwise treat as EnvFilter string
            // Try constructing an EnvFilter to validate
            EnvFilter::try_new(s)
                .map(|_| s.to_string())
                .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid log filter: {}", e)))
        }
    }
}

pub fn set_log_level(level_or_filter: &str) -> Result<(), WasmSdkError> {
    let filter_str = normalize_level_or_filter(level_or_filter)?;

    if let Some(handle) = RELOAD_HANDLE.get() {
        // Update existing filter
        handle
            .modify(|f| *f = EnvFilter::new(filter_str.clone()))
            .map_err(|e| WasmSdkError::generic(format!("Failed to update log filter: {}", e)))?;
        return Ok(());
    }

    // Initialize subscriber for the first time
    let env_filter = EnvFilter::new(filter_str);
    let (layer, handle) = reload::Layer::new(env_filter);
    let wasm_layer = tracing_wasm::WASMLayer::new(tracing_wasm::WASMLayerConfig::default());

    let subscriber = Registry::default().with(layer).with(wasm_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| WasmSdkError::generic(format!("Failed to set global logger: {}", e)))?;

    let _ = RELOAD_HANDLE.set(handle);
    Ok(())
}

