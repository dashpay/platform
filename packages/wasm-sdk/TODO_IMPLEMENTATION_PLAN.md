# TODO Implementation Plan

This document provides an actionable plan for addressing the 44 TODOs in the WASM SDK codebase.

## Executive Summary

Of the 44 TODOs:
- **20 can be implemented now** with existing infrastructure
- **15 are blocked** by platform dependencies
- **9 are nice-to-have** improvements

## Immediate Implementation Opportunities

### 1. Withdrawal Operations (5 TODOs) - HIGH PRIORITY
**File**: `src/withdrawal.rs`
**Why Important**: User funds management

```rust
// Implementation approach:
pub async fn create_withdrawal(
    sdk: &WasmSdk,
    identity_id: &str,
    amount: u64,
    core_address: &str,
    signer: &WasmSigner,
) -> Result<JsValue, JsError> {
    // 1. Validate core address format
    // 2. Create withdrawal document
    // 3. Sign with identity key
    // 4. Broadcast via DAPI client
}
```

**Tasks**:
- [ ] Implement `create_withdrawal()` using document creation pattern
- [ ] Implement `get_withdrawal_status()` using document fetch
- [ ] Implement `list_withdrawals()` using document query
- [ ] Implement `broadcast_withdrawal()` using existing broadcast
- [ ] Add base58check validation utility

### 2. Deserialization Methods (3 TODOs) - HIGH PRIORITY
**File**: `src/serializer.rs`
**Why Important**: Complete round-trip serialization

```rust
// Already have serialize, need deserialize:
- deserializeStateTransition()
- deserializeDocument()  
- deserializeIdentity()
```

**Implementation**: Follow existing patterns in the file, use DPP deserialization

### 3. Unproved Data Fetching (2 TODOs) - MEDIUM PRIORITY
**File**: `src/fetch_unproved.rs`
**Why Important**: Performance optimization

```rust
pub async fn fetch_identity_unproved(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<JsValue, JsError> {
    let client = sdk.get_dapi_client()?;
    client.get_identity_unproved(identity_id).await
}
```

### 4. Batch Operations (2 TODOs) - MEDIUM PRIORITY
**File**: `src/fetch_many.rs`
**Why Important**: Performance improvement

```rust
// Use Promise.all pattern or concurrent futures
pub async fn fetch_many_identities(
    sdk: &WasmSdk,
    identity_ids: Vec<String>,
) -> Result<JsValue, JsError> {
    let client = sdk.get_dapi_client()?;
    let futures = identity_ids.into_iter()
        .map(|id| client.get_identity(&id));
    // Collect results
}
```

### 5. Group Actions - State Transitions (6 TODOs) - LOW PRIORITY
**File**: `src/group_actions.rs`
**Why Important**: New feature

Pattern for each:
```rust
pub async fn create_group(
    sdk: &WasmSdk,
    owner_id: &str,
    name: String,
    members: Vec<String>,
    threshold: u32,
    signer: &WasmSigner,
) -> Result<JsValue, JsError> {
    // 1. Create group document structure
    // 2. Create state transition
    // 3. Sign and broadcast
}
```

## Blocked TODOs (Cannot Implement Yet)

### 1. Platform Proto Dependencies (7 TODOs)
**Blocker**: Need platform_proto WASM support
- Response parsing in broadcast.rs
- Group info in state_transitions/group.rs

### 2. API Limitations (4 TODOs)
**Blocker**: Platform API doesn't expose these yet
- Group info getters/setters
- Proof verification details

### 3. External Library Support (4 TODOs)
**Blocker**: Libraries not WASM-compatible
- BIP39 wordlist validation
- Base58check implementation
- WebSocket subscription enhancements

## Implementation Priority Matrix

| Priority | Effort | TODOs | Files |
|----------|--------|-------|-------|
| HIGH | Low | 5 | serializer.rs (3), fetch_unproved.rs (2) |
| HIGH | Medium | 5 | withdrawal.rs (5) |
| MEDIUM | Low | 2 | fetch_many.rs (2) |
| MEDIUM | Medium | 8 | group_actions.rs (6), monitoring (2) |
| LOW | Low | 9 | Various improvements |
| BLOCKED | - | 15 | Platform dependencies |

## Recommended Implementation Order

### Sprint 1 (1 week)
1. **Deserializers** - Complete serialization story
2. **Unproved fetching** - Quick wins
3. **Batch operations** - Performance boost

### Sprint 2 (1 week)
1. **Withdrawal operations** - Critical user feature
2. **Basic monitoring** - Using setInterval

### Sprint 3 (2 weeks)
1. **Group actions** - New feature set
2. **Enhanced validation** - Security improvements

### Future (When Unblocked)
1. Platform proto integration
2. Advanced proof verification
3. WebSocket enhancements

## Code Examples for Common Patterns

### Pattern 1: DAPI Client Usage
```rust
let config = DapiClientConfig::new(sdk.network());
let client = DapiClient::new(config)?;
let result = client.some_method(params).await?;
```

### Pattern 2: State Transition Creation
```rust
let mut st_bytes = Vec::new();
st_bytes.push(TRANSITION_TYPE);
st_bytes.extend_from_slice(&data);
// Sign and broadcast
```

### Pattern 3: Document Operations
```rust
let doc = create_document(
    sdk,
    contract_id,
    owner_id,
    doc_type,
    data,
    signer
).await?;
```

## Testing Strategy

For each implemented TODO:
1. Add unit test in corresponding test file
2. Add integration test if cross-module
3. Update documentation
4. Remove TODO comment

## Success Metrics

- Reduce TODO count from 44 to under 20
- All critical user operations implemented
- No security-related TODOs remaining
- Clear documentation for blocked items

## Next Actions

1. Create GitHub issues for each TODO category
2. Assign developers to Sprint 1 tasks
3. Set up tracking dashboard
4. Schedule platform team sync for blocked items