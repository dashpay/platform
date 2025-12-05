# wasm-dpp2 / wasm-sdk – Serde Format Context for JSON/WASM Serialization

This document describes the **format-aware serialization** system that allows types like `IdentifierWasm` to:

- Serialize to **bytes** (Uint8Array) for the WASM/object path via `serde_wasm_bindgen`
- Serialize to **string** (Base58) for the JSON path via `serde_json`

using a single `Serialize`/`Deserialize` impl that branches based on a **thread-local format context**.

---

## Architecture Overview

The solution uses a **thread-local context** pattern instead of serializer wrappers:

1. A thread-local `SerdeFormat` enum stores the current serialization context (`Json` or `Wasm`)
2. Helper functions set this context before calling the underlying serializers
3. Custom `Serialize`/`Deserialize` impls read `current_format()` to decide their behavior

### Why Thread-Local?

The original design proposed wrapping serializers and using `Any` downcasting to detect the format. This **cannot work** because:

- Serde's `Serializer` trait doesn't have an `Any` bound
- Generic type parameters are erased at compile time
- Nested serializers (for sequences, maps, etc.) lose the wrapper type

Thread-local works safely in WASM because:

- WebAssembly is single-threaded by default
- JavaScript's event loop ensures only one JS→WASM call executes at a time
- No preemption means a serialization call runs to completion before another can start

---

## 1. The `serde_format` Module

**Location:** `wasm-dpp2/src/serde_format.rs`

### 1.1. SerdeFormat Enum

```rust
/// Serialization format context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerdeFormat {
    /// JSON serialization: identifiers become Base58 strings, etc.
    Json,
    /// WASM/object serialization: identifiers become Uint8Array bytes, etc.
    #[default]
    Wasm,
}
```

### 1.2. Thread-Local Context

```rust
use std::cell::Cell;

thread_local! {
    static SERDE_FORMAT: Cell<SerdeFormat> = const { Cell::new(SerdeFormat::Wasm) };
}

/// Returns the current serialization format context.
/// Defaults to `SerdeFormat::Wasm` if no context has been set.
pub fn current_format() -> SerdeFormat {
    SERDE_FORMAT.with(|f| f.get())
}
```

### 1.3. RAII Format Guard

The `FormatGuard` ensures the format is restored even if serialization panics:

```rust
struct FormatGuard {
    previous: SerdeFormat,
}

impl FormatGuard {
    fn new(format: SerdeFormat) -> Self {
        let previous = SERDE_FORMAT.with(|f| {
            let prev = f.get();
            f.set(format);
            prev
        });
        FormatGuard { previous }
    }
}

impl Drop for FormatGuard {
    fn drop(&mut self) {
        SERDE_FORMAT.with(|f| f.set(self.previous));
    }
}
```

### 1.4. Helper Functions

```rust
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;

/// Serialize to serde_json::Value with JSON format context.
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsonValue, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::to_value(value)
}

/// Deserialize from serde_json::Value with JSON format context.
pub fn from_json_value<T: DeserializeOwned>(value: JsonValue) -> Result<T, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::from_value(value)
}

/// Serialize to JsValue with WASM format context.
pub fn to_wasm_value<T: Serialize>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Wasm);
    serde_wasm_bindgen::to_value(value)
}

/// Deserialize from JsValue with WASM format context.
pub fn from_wasm_value<T: DeserializeOwned>(js: JsValue) -> Result<T, serde_wasm_bindgen::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Wasm);
    serde_wasm_bindgen::from_value(js)
}
```

Additional convenience functions are available:

- `to_json_string()` / `to_json_string_pretty()` / `from_json_str()`

---

## 2. Implementing Format-Aware Serialize/Deserialize

Types that need different serialization for JSON vs WASM check `current_format()`:

### Example: IdentifierWasm

```rust
use crate::serde_format::{current_format, SerdeFormat};
use serde::{Serialize, Serializer, Deserialize, Deserializer};

impl Serialize for IdentifierWasm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match current_format() {
            SerdeFormat::Json => {
                // JSON: serialize as Base58 string
                serializer.serialize_str(&self.to_base58())
            }
            SerdeFormat::Wasm => {
                // WASM: serialize as bytes (becomes Uint8Array)
                serializer.serialize_bytes(&self.0.to_vec())
            }
        }
    }
}

impl<'de> Deserialize<'de> for IdentifierWasm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match current_format() {
            SerdeFormat::Json => {
                // JSON: expect Base58 string
                let s = String::deserialize(deserializer)?;
                IdentifierWasm::try_from(s.as_str()).map_err(D::Error::custom)
            }
            SerdeFormat::Wasm => {
                // WASM: expect bytes or any compatible representation
                deserializer.deserialize_any(IdentifierWasmVisitor)
            }
        }
    }
}
```

---

## 3. Using in wasm-sdk

The `wasm-sdk` crate's `serialization.rs` module wraps the `serde_format` helpers:

```rust
use wasm_dpp2::serde_format;

/// Serialize to JsValue with WASM format context.
/// Types like `IdentifierWasm` will serialize as bytes (Uint8Array).
pub fn to_object<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    serde_format::to_wasm_value(value)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Serialize to JsValue with JSON format context.
/// Types like `IdentifierWasm` will serialize as Base58 strings.
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    let json = serde_format::to_json_value(value)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&json)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))
}
```

### The `impl_wasm_object_json!` Macro

This macro generates `toObject`, `fromObject`, `toJSON`, and `fromJSON` methods for WASM types:

```rust
impl_wasm_object_json!(MyTypeWasm);
```

Generates:

- `toObject()` → uses WASM format (bytes as Uint8Array)
- `fromObject()` → uses WASM format
- `toJSON()` → uses JSON format (identifiers as strings)
- `fromJSON()` → uses JSON format

---

## 4. Adding Format-Aware Serialization to New Types

To make a new type format-aware:

1. Import the context functions:

   ```rust
   use crate::serde_format::{current_format, SerdeFormat};
   ```

2. Implement `Serialize` with format branching:

   ```rust
   impl Serialize for MyType {
       fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
       where
           S: Serializer,
       {
           match current_format() {
               SerdeFormat::Json => {
                   // Human-readable representation
               }
               SerdeFormat::Wasm => {
                   // Binary/efficient representation
               }
           }
       }
   }
   ```

3. Implement `Deserialize` similarly if needed.

---

## 5. Thread Safety Notes

### Safe in Standard WASM

The thread-local approach is safe because:

- WASM in browsers/Node.js is single-threaded
- JavaScript's event loop prevents concurrent WASM calls
- Serialization runs to completion before another call can start

### Exception: SharedArrayBuffer + Web Workers

If using `wasm-bindgen-rayon` or SharedArrayBuffer with multiple workers, you'd have real concurrency. In that case, consider:

- Using a different context mechanism (e.g., explicit parameter passing)
- Ensuring each worker has its own thread-local context (which they do by default)

---

## 6. Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| `SerdeFormat` | `wasm-dpp2/src/serde_format.rs` | Enum: `Json` or `Wasm` |
| `current_format()` | `wasm-dpp2/src/serde_format.rs` | Read current context |
| `FormatGuard` | `wasm-dpp2/src/serde_format.rs` | RAII context setter |
| `to_json_value()` | `wasm-dpp2/src/serde_format.rs` | JSON-context serialization |
| `to_wasm_value()` | `wasm-dpp2/src/serde_format.rs` | WASM-context serialization |
| `to_object()` | `wasm-sdk/src/serialization.rs` | SDK wrapper for WASM format |
| `to_json_value()` | `wasm-sdk/src/serialization.rs` | SDK wrapper for JSON format |

This design provides:

- ✅ Single `Serialize`/`Deserialize` impl per type
- ✅ Clean separation between JSON (human-readable) and WASM (binary) formats
- ✅ Automatic context propagation through nested structures
- ✅ Panic-safe context cleanup via RAII guard
- ✅ No changes to `is_human_readable()` semantics
