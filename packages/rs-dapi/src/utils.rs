use std::time::UNIX_EPOCH;

static JSONRPC_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn generate_jsonrpc_id() -> String {
    let timestamp = UNIX_EPOCH.elapsed().unwrap_or_default().as_nanos();

    let pid = std::process::id();
    let counter = JSONRPC_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    format!("{pid}-{counter}-{timestamp}")
}
