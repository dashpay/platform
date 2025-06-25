# Structured Logging

The wasm-drive-verify library includes a structured logging system for debugging verification operations.

## Enabling Logging

Logging is disabled by default to minimize bundle size. To enable logging, compile with the `debug_logs` feature:

```bash
# Build with logging enabled
wasm-pack build -- --features debug_logs

# Or with cargo
cargo build --features debug_logs
```

## Log Levels

The logging system supports the following levels:
- **TRACE**: Detailed trace information
- **DEBUG**: Debug information for development
- **INFO**: Informational messages
- **WARN**: Warning messages
- **ERROR**: Error messages

## Usage in Code

### Basic Logging

```rust
use crate::utils::logging::{debug, error, info, warn};

// Log simple messages
debug("module_name", "Starting verification");
info("module_name", format!("Processing {} items", count));
warn("module_name", "Deprecated function called");
error("module_name", "Verification failed");
```

### Logging with Context

```rust
use crate::utils::logging::{log_with_context, LogLevel};

// Log with additional context
log_with_context(
    LogLevel::Debug,
    "module_name",
    "Processing identity",
    JsValue::from_str(&format!("ID: {}", id))
);
```

### Performance Logging

```rust
use crate::utils::logging::PerfLogger;

// Automatically logs start and completion with timing
let _perf = PerfLogger::new("module_name", "expensive_operation");
// ... do work ...
// Logs completion time when _perf goes out of scope
```

### Using Macros

```rust
// Debug logging macro
log_debug!("module_name", "Debug message");
log_debug!("module_name", "Debug with context", context_value);

// Error logging macro
log_error!("module_name", "Error message");
log_error!("module_name", "Error with context", error_details);
```

## Browser Console Output

When enabled, logs appear in the browser console with structured formatting:

```
[identity] DEBUG: Starting verification
[identity] DEBUG: Verifying identity with proof size: 1024 bytes
[identity] DEBUG: Completed: verify_full_identity_by_identity_id (took 5.23ms)
[identity] ERROR: Verification failed: InvalidProof
```

## Performance Considerations

- Logging is completely compiled out when the `debug_logs` feature is not enabled
- No runtime overhead when disabled
- Minimal overhead when enabled (browser console calls)
- Performance timing uses browser's high-resolution `performance.now()` API

## Best Practices

1. **Module Names**: Use consistent module names (e.g., "identity", "document", "contract")
2. **Log Levels**: Use appropriate levels (debug for development, error for failures)
3. **Performance**: Use PerfLogger for operations that might be slow
4. **Context**: Include relevant context in error messages
5. **Sensitive Data**: Never log private keys or sensitive user data

## Example Integration

```rust
pub fn verify_something(
    proof: &Uint8Array,
    id: &Uint8Array,
) -> Result<VerifyResult, JsValue> {
    let _perf = PerfLogger::new("module", "verify_something");
    
    debug("module", format!("Starting verification with {} byte proof", proof.length()));
    
    let result = perform_verification(proof, id)
        .map_err(|e| {
            error("module", format!("Verification failed: {:?}", e));
            format_error(ErrorCategory::VerificationError, "Verification failed")
        })?;
    
    debug("module", "Verification completed successfully");
    Ok(result)
}
```