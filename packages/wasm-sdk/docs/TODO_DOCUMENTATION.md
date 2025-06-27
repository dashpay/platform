# TODO Documentation for WASM SDK

This document explains the TODO items found in the codebase and why they are deferred.

## verify.rs TODOs

### 1. DriveDocumentQuery Creation (Lines 177, 186)
```rust
// TODO: Create proper DriveDocumentQuery when drive types are available
```

**Status**: Deferred
**Reason**: The `DriveDocumentQuery` type is part of the `rs-drive` crate which is being refactored in the Platform v1.0 release. Once the drive types are stabilized and exposed for WASM usage, this can be implemented properly.

**Current Implementation**: Returns an empty/mock query that satisfies the interface but doesn't perform actual queries.

**Impact**: Document verification functionality is limited but the interface is stable for future implementation.

### 2. InternalClauses Creation (Line 243)
```rust
// TODO: Return proper InternalClauses when drive types are available
```

**Status**: Deferred
**Reason**: Similar to DriveDocumentQuery, the `InternalClauses` type depends on drive internals that aren't yet exposed for WASM.

**Current Implementation**: Returns empty clauses that maintain API compatibility.

**Impact**: Where clause functionality in document queries is limited to basic operations.

### 3. DriveQuery Types (Lines 268, 351, 356)
```rust
// TODO: Implement when rs-drive types are available for WASM
```

**Status**: Deferred
**Reason**: These require the full `rs-drive` query system which includes:
- Complex query builders
- Indexed query optimization
- Proof path construction

**Current Implementation**: Simplified query structures that handle basic use cases.

**Impact**: Advanced query features like complex joins and optimized indexing are not available.

## state_transitions/group.rs TODOs

### 1. Group Info API (Lines 325, 330)
```rust
// TODO: When group info API is available, validate group exists
// TODO: Validate member has required permissions
```

**Status**: Awaiting Platform Feature
**Reason**: The group management system is still being finalized in Platform v1.0. The APIs for:
- Group membership validation
- Permission checking
- Group state queries

Are not yet available in the platform RPC interface.

**Current Implementation**: Basic validation only - checks data structure validity without group-specific rules.

**Impact**: Group actions can be created but full validation happens only on the platform side.

### 2. Group Action Validation (Lines 351, 356)
```rust
// TODO: Implement proper validation when group system is complete
```

**Status**: Awaiting Platform Feature
**Reason**: Requires:
- Group consensus rules
- Multi-signature validation
- Threshold checking logic

These are being implemented as part of the Platform's governance system.

**Current Implementation**: Structural validation only.

**Impact**: Group proposals and actions can be created but complex validation rules aren't enforced client-side.

## Resolution Timeline

1. **Drive Types** (verify.rs): Expected in Platform v1.1 when drive crate is refactored for WASM compatibility
2. **Group System** (group.rs): Expected in Platform v1.0 final release with full governance features

## Mitigation Strategy

For developers using these features:

1. **Document Queries**: Use simple where clauses and expect basic functionality
2. **Group Actions**: Rely on platform-side validation and handle errors appropriately
3. **Migration Path**: The APIs are designed to be forward-compatible, so code written today will work when full implementation is available

## Testing Strategy

Despite incomplete implementations, these modules have:
- Interface tests to ensure API stability
- Mock tests to verify expected behavior
- Integration tests that will automatically validate against real implementation when available