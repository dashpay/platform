# Getter Method Optimization

This document describes the optimization applied to getter methods to avoid unnecessary cloning.

## Problem

Previously, getter methods for `Vec<u8>` fields were cloning the data:

```rust
#[wasm_bindgen(getter)]
pub fn root_hash(&self) -> Vec<u8> {
    self.root_hash.clone()  // Unnecessary clone!
}
```

## Solution

We now return `Uint8Array` views directly without cloning the underlying data:

```rust
use crate::utils::getters::VecU8ToUint8Array;

#[wasm_bindgen(getter)]
pub fn root_hash(&self) -> Uint8Array {
    self.root_hash.to_uint8array()  // No clone, just a view!
}
```

## Implementation

The optimization is provided by the `VecU8ToUint8Array` trait in `utils/getters.rs`:

```rust
pub trait VecU8ToUint8Array {
    fn to_uint8array(&self) -> js_sys::Uint8Array;
}
```

This trait is implemented for `Vec<u8>` and `[u8]` to create JavaScript `Uint8Array` views without cloning.

## Benefits

1. **Memory Efficiency**: No unnecessary cloning of byte arrays
2. **Performance**: Faster getter calls, especially for large arrays
3. **JavaScript Compatibility**: Returns native `Uint8Array` which is more idiomatic for JavaScript consumers

## Migration

To optimize a getter method:

1. Add import: `use crate::utils::getters::VecU8ToUint8Array;`
2. Change return type from `Vec<u8>` to `Uint8Array`
3. Replace `self.field.clone()` with `self.field.to_uint8array()`

## Notes

- This optimization was applied to 51 getter methods across the codebase
- For `JsValue` fields, cloning is kept as it's already reference-counted and cheap
- The optimization maintains the same API contract from JavaScript's perspective