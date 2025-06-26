# TODO Analysis for WASM SDK

This document analyzes all TODO comments in the codebase and categorizes them by priority and feasibility.

## Summary

- **Total TODOs**: 44
- **Files with TODOs**: 16
- **Most TODOs**: group_actions.rs (11)

## Categorization

### 1. Blocked by External Dependencies (High Priority)

These TODOs are blocked by missing platform features or external dependencies:

#### Platform Proto / API Limitations
- `state_transitions/group.rs`: 4 TODOs waiting for group info API
  - Cannot set/get group info on transitions until API is available
- `broadcast.rs`: 2 TODOs waiting for platform_proto types
  - Cannot parse responses without protobuf definitions
- `verify.rs`: 1 TODO waiting for wasm-drive-verify to expose proof verification

#### WebSocket Support
- `dapi_client/mod.rs`: 1 TODO for WebSocket subscriptions
  - Already implemented basic WebSocket, but needs platform support

### 2. Implementable Now (Medium Priority)

These TODOs could be implemented with current technology:

#### State Transition Creation
- `group_actions.rs`: 6 TODOs for state transition creation
  - Group creation, member management, proposals, voting
  - These follow similar patterns to existing state transitions
- `withdrawal.rs`: 4 TODOs for withdrawal operations
  - Create, broadcast, status checking
  - Similar to other state transition implementations

#### Data Fetching
- `fetch_unproved.rs`: 2 TODOs for DAPI client calls
- `fetch_many.rs`: 2 TODOs for batch fetching
- `prefunded_balance.rs`: 1 TODO for balance fetching
- All can use the existing DAPI client

#### Monitoring Features
- `contract_history.rs`: 2 TODOs for monitoring
- `identity_info.rs`: 1 TODO for monitoring with web workers
- `prefunded_balance.rs`: 1 TODO for balance monitoring
- Can be implemented with setInterval or web workers

### 3. Nice to Have (Low Priority)

These TODOs are for improvements or optimizations:

#### Validation Enhancements
- `signer.rs`: 2 TODOs for BIP39 validation
  - Wordlist validation and checksum validation
  - Would improve security but basic validation exists
- `withdrawal.rs`: 1 TODO for base58check validation
- `broadcast.rs`: 1 TODO for additional validation

#### Schema Analysis
- `contract_cache.rs`: 1 TODO for analyzing contract references
  - Would improve caching efficiency

#### Deserialization
- `serializer.rs`: 3 TODOs for deserialization methods
  - Currently only serialization is implemented

#### Context Provider
- `context_provider.rs`: 1 TODO for token configuration
  - Nice to have for token features

## Detailed Analysis by File

### group_actions.rs (11 TODOs)
```rust
// State transition creation (6)
- create_group() 
- add_group_member()
- remove_group_member()
- create_group_proposal()
- vote_on_proposal()
- execute_group_action()

// Data fetching (4)
- get_group_info()
- get_group_members()
- get_group_proposals()
- get_group_permissions()

// Permission checking (1)
- check_group_permission()
```

### withdrawal.rs (5 TODOs)
```rust
- create_withdrawal()
- get_withdrawal_status()
- list_withdrawals()
- broadcast_withdrawal()
- validate_core_withdrawal_address() // base58check
```

### state_transitions/group.rs (5 TODOs)
```rust
- Deserialize GroupActionEvent (1)
- Set/get group info on transitions (4) - blocked by API
```

### Others (23 TODOs)
Various implementation tasks across other files.

## Implementation Priority

### Phase 1: Complete Existing Features
1. **Deserialization methods** (serializer.rs) - Complete the serialization story
2. **Unproved fetching** (fetch_unproved.rs) - Use existing DAPI client
3. **Batch operations** (fetch_many.rs) - Performance improvement

### Phase 2: New Features
1. **Group actions** - Major feature addition
2. **Withdrawal operations** - Important for user funds
3. **Enhanced monitoring** - Better observability

### Phase 3: Platform Dependencies
1. Wait for platform proto WASM support
2. Wait for group info API
3. Wait for proof verification API

## Recommendations

1. **Document Workarounds**: For blocked TODOs, document temporary solutions
2. **Create Issues**: Convert high-priority TODOs to GitHub issues
3. **Remove Stale TODOs**: Some TODOs might be outdated after recent implementations
4. **Add Context**: Some TODOs lack context about why they're blocked

## Code Quality Impact

Most TODOs represent missing features rather than technical debt. The codebase is well-structured to add these features when dependencies are available.

### Risk Assessment
- **High Risk**: 0 TODOs (no security or stability issues)
- **Medium Risk**: 11 TODOs (missing core features like withdrawals)
- **Low Risk**: 33 TODOs (nice-to-have features)

## Next Steps

1. Prioritize implementing withdrawal operations (user funds)
2. Complete deserialization for round-trip support
3. Implement group actions as they're a major platform feature
4. Create a roadmap for platform-dependent features