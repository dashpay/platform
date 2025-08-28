# Address Discovery Limitations with Skip Sync Configuration

## Overview

This document explains critical limitations in HD wallet address discovery when using `skipSynchronizationBeforeHeight` and provides solutions for high-index address discovery.

## The Problem

### Default Behavior
- **Initial address generation**: 40 addresses (20 external + 20 internal)
- **BIP44_ADDRESS_GAP**: 20 (defined in `/src/CONSTANTS.js`)
- **Gap management**: Generates 20 more addresses after last used address

### Skip Sync Impact on Address Discovery

When using `skipSynchronizationBeforeHeight`:
```javascript
wallet: {
  unsafeOptions: {
    skipSynchronizationBeforeHeight: 1308950
  }
}
```

**Critical Issue**: Addresses with transactions **before** the skip height cannot be discovered as "used" because:

1. Wallet only syncs from block 1308950 forward
2. Historical transactions are not processed  
3. Addresses 0-N appear "unused" to the system
4. Gap management never triggers to generate higher-index addresses
5. **Result**: Addresses beyond index ~40 are never generated or discovered

## Real-World Example

**Scenario**: User has address at index 137
- **Created**: After block 1308950 (so it should be discoverable)
- **Problem**: Wallet generates only indices 0-39 initially
- **Gap logic**: Cannot reach index 137 because indices 0-136 appear unused
- **Outcome**: Address at index 137 is never generated, never added to bloom filter, never discovered

## Technical Deep Dive

### Address Generation Logic (`ensureAccountAddressesToGapLimit`)

```javascript
// Current logic in ensureAddressesToGapLimit.js
const gapBetweenLastUsedAndLastGenerated = {
  external: lastGeneratedIndexes.external - lastUsedIndexes.external,
  internal: lastGeneratedIndexes.internal - lastUsedIndexes.internal,
};

const addressesToGenerate = {
  external: BIP44_ADDRESS_GAP - gapBetweenLastUsedAndLastGenerated.external,
  internal: BIP44_ADDRESS_GAP - gapBetweenLastUsedAndLastGenerated.internal,
};
```

**The Mathematics**:
- Initial: generates indices 0-19 (gap = 20)
- If no addresses marked as "used": gap remains 20, no new generation
- To reach index 137: need ~7 gap expansions (137 ÷ 20 ≈ 7)
- **But**: requires detecting usage of indices 0, 20, 40, 60, 80, 100, 120 first

### Skip Sync Breaks the Chain

With `skipSynchronizationBeforeHeight=1308950`:
1. Addresses with historical transactions are never marked as "used"
2. Gap calculation remains at initial state
3. No trigger for additional address generation
4. High-index addresses remain undiscovered

## Solutions

### Solution 1: Increase BIP44_ADDRESS_GAP (Recommended)

**File**: `/packages/wallet-lib/src/CONSTANTS.js`

```javascript
module.exports = {
  // ... other constants
  BIP44_ADDRESS_GAP: 150, // Increased from 20 to cover high indices
  // ... rest of constants
};
```

**Benefits**:
- ✅ Initial generation covers indices 0-149 (300 addresses total)
- ✅ Works with skip sync configuration
- ✅ No code changes needed in applications
- ✅ Covers most real-world usage patterns

**Trade-offs**:
- ⚠️ Larger bloom filter (300 vs 40 addresses)
- ⚠️ Slightly slower initial sync
- ⚠️ More memory usage for address tracking

### Solution 2: Conditional Gap Increase

**File**: `/packages/wallet-lib/src/types/Account/Account.js`

Add dynamic gap based on skip sync configuration:

```javascript
// In Account constructor, after line 43
const dynamicGap = this.skipSyncHeight ? 
  Math.max(BIP44_ADDRESS_GAP, 150) : // Use larger gap with skip sync
  BIP44_ADDRESS_GAP; // Use normal gap for full sync

keyChainStoreOpts.lookAheadOpts = {
  paths: {
    'm/0': dynamicGap,
    'm/1': dynamicGap,
  },
};
```

### Solution 3: Pre-Generation for Known High Indices

For applications that know they use high-index addresses:

```javascript
// Before account sync, manually generate addresses
const maxKnownIndex = 150; // Known highest address index
for (let i = 40; i <= maxKnownIndex; i++) {
  account.generateAddress(i, 'external');
  account.generateAddress(i, 'internal');
}
```

### Solution 4: Two-Phase Discovery

1. **Phase 1**: Full sync to discover all used addresses (slow)
2. **Phase 2**: Use skip sync for subsequent operations (fast)

```javascript
// First run: discover all addresses
const discoveryWallet = new Dash.Client({
  network: 'testnet',
  wallet: { mnemonic: MNEMONIC }  // No skip sync
});

// Subsequent runs: use skip sync
const fastWallet = new Dash.Client({
  network: 'testnet', 
  wallet: {
    mnemonic: MNEMONIC,
    unsafeOptions: { skipSynchronizationBeforeHeight: 1308950 }
  }
});
```

## Configuration Recommendations

### For High-Volume Wallets
```javascript
// Recommended for wallets with >100 addresses
BIP44_ADDRESS_GAP: 200,
```

### For Development/Testing  
```javascript
// Recommended for development with known address ranges
BIP44_ADDRESS_GAP: 150,
```

### For Production Applications
```javascript
// Conservative approach - initial discovery run
BIP44_ADDRESS_GAP: 100,
// Then use skip sync for performance
```

## Detection and Debugging

### Check Current Address Range
```javascript
const addresses = account.storage.getWalletStore(account.walletId).addresses;
console.log('External addresses:', Object.keys(addresses.external).length);
console.log('Internal addresses:', Object.keys(addresses.internal).length);
console.log('Highest external index:', Math.max(...Object.keys(addresses.external)
  .map(path => parseInt(path.split('/')[5]))));
```

### Verify Address Generation
```javascript
// Check if specific index exists
const hasAddress137 = account.getAddress(137, 'external');
console.log('Address 137 exists:', !!hasAddress137);
```

## Performance Considerations

### Bloom Filter Size Impact

| BIP44_ADDRESS_GAP | Total Addresses | Bloom Filter Size | Performance Impact |
|-------------------|-----------------|-------------------|-------------------|
| 20 (default)      | 40             | ~160 bytes        | Baseline          |
| 50                | 100            | ~400 bytes        | Minimal           |
| 100               | 200            | ~800 bytes        | Low               |
| 150               | 300            | ~1.2KB           | Low               |
| 200               | 400            | ~1.6KB           | Moderate          |

**Recommendation**: BIP44_ADDRESS_GAP=150 provides good balance between discovery capability and performance.

## Key Insights

1. **Skip sync breaks address discovery** for addresses with historical transactions
2. **BIP44_ADDRESS_GAP is the primary limiting factor** for high-index discovery
3. **Sequential gap assumption** prevents discovery of sparse address usage patterns
4. **Bloom filter efficiency** requires pre-generation of addresses to monitor
5. **No automatic discovery** of addresses beyond gap limit without usage triggers

## Action Items for Developers

### Immediate Fix for Index 137
1. Increase `BIP44_ADDRESS_GAP` to 150+ in CONSTANTS.js
2. Restart wallet application to regenerate with new gap
3. Verify address discovery in debug logs

### Long-term Architecture Improvements
1. Consider configurable gap limits per application
2. Implement address range scanning for recovery scenarios
3. Add discovery hints for known high-index usage patterns
4. Optimize bloom filter management for large address sets

---

**Date Created**: 2025-08-28  
**Context**: feat-disable-id-worker branch  
**Related Issue**: Address at index 137 not discovered with skipSynchronizationBeforeHeight=1308950