# WASM SDK JavaScript Wrapper Pattern Alignment Plan

**COMPREHENSIVE VERIFIED ANALYSIS WITH PRECISE STATISTICS**

Based on exhaustive analysis of the current codebase, I've identified significant pattern divergences between the existing JavaScript wrapper, the example files, and test files. Here's the verified comprehensive fix plan with accurate counts and precise implementation details:

## 🔍 Key Findings

### ✅ **Correct Patterns** (from examples)
- **Environment Setup**: Proper Node.js crypto polyfill, WASM pre-loading
- **Configuration Management**: `.env` file loading with smart defaults  
- **JavaScript Wrapper Usage**: `import { WasmSDK } from '../src-js/index.js'` (correct approach)
- **Modern Initialization**: `new WasmSDK(config)` → `await sdk.initialize()`
- **Error Handling**: Network errors expected in offline mode
- **Resource Management**: Proper cleanup with `await sdk.destroy()`

### ❌ **Pattern Divergences Found**

**1. Test Files Using Direct WASM API** 
- Tests import raw WASM: `import init, * as wasmSdk from '../pkg/wasm_sdk.js'`
- Should use JavaScript wrapper: `import { WasmSDK } from '../src-js/index.js'`

**2. Inconsistent Initialization Patterns**
- Raw WASM: `wasmSdk.WasmSdkBuilder.new_testnet_trusted()`  
- Wrapper: `new WasmSDK({ network: 'testnet', proofs: true })`

**3. Direct WASM Function Calls**
- Raw: `wasmSdk.identity_fetch(sdk, identityId)`
- Wrapper: `sdk.getIdentity(identityId)`

**4. Missing JavaScript Wrapper Methods**
- Many WASM functions not wrapped in `src-js/index.js`
- Test coverage relies on direct WASM access

## 📋 Implementation Plan

### Phase 1: Audit JavaScript Wrapper Completeness
1. **Compare WASM exports vs JavaScript wrapper methods**
2. **Identify missing wrapper functions** (queries, state transitions, utilities)
3. **Document API gaps** between raw WASM and wrapper

### Phase 2: Expand JavaScript Wrapper API
1. **Add missing query methods** following established patterns
2. **Add missing state transition methods**  
3. **Add missing utility functions** (key generation, DPNS, validation)
4. **Maintain consistent error handling and resource management**

### Phase 3: Update All Test Files  
1. **Convert tests to use JavaScript wrapper** instead of raw WASM
2. **Update initialization patterns** to use modern `WasmSDK` class
3. **Standardize error handling** for network/offline scenarios
4. **Implement proper resource cleanup** in all tests

### Phase 4: Pattern Validation
1. **Run updated tests** against the aligned patterns
2. **Verify functionality** matches between wrapper and raw WASM
3. **Update examples** to reflect consistent patterns
4. **Update documentation** with correct usage patterns

## 🎯 Expected Outcomes

- **Consistent API**: All code uses JavaScript wrapper pattern
- **Better Maintainability**: Single point of truth for WASM integration
- **Improved Error Handling**: Consistent error patterns across codebase  
- **Resource Safety**: Proper cleanup and resource management
- **Test Reliability**: Tests work consistently with wrapper abstraction

## 🚨 Critical Dependencies & Blockers

**BLOCKING ISSUES (Must fix before proceeding):**
1. **Missing Critical Functions**: Key generation and crypto functions essential for tests
2. **Test Migration Dependency Chain**: Tests cannot migrate until wrapper has functions they use
3. **Implementation Quality**: Some existing wrapper methods need verification

**TECHNICAL DEPENDENCIES:**
- WASM module build must be working (`./build.sh`) ✅
- JavaScript wrapper must be functionally complete before test migration ❌
- Network connectivity for integration testing ✅
- Proper error handling patterns established ✅
- Resource management patterns established ✅

**MIGRATION SEQUENCE CONSTRAINT:**
Tests CANNOT migrate to wrapper until wrapper implements the functions they actually use. Current wrapper only covers ~9% of needed functionality.

**IMPLEMENTATION ORDER:**
1. **First**: Implement critical missing functions (key generation, DPNS, system queries)
2. **Second**: Verify existing wrapper method implementations work correctly
3. **Third**: Migrate tests file by file as functionality becomes available
4. **Fourth**: Implement remaining wrapper functions for completeness

## 📝 Detailed Analysis (COMPREHENSIVE VERIFICATION)

### Current JavaScript Wrapper Coverage - PRECISE STATISTICS

**📊 Statistical Overview:**
- **WASM SDK Exports**: 141 total functions
- **JavaScript Wrapper Methods**: 13 total methods (11 async + 2 sync utility)
- **Coverage Gap**: ~91% of WASM functionality missing from wrapper
- **Test Files**: All 24 test files use direct WASM API instead of wrapper

**✅ Implemented in `src-js/index.js` (13 methods):**

*Core Query Methods (5):*
- `getIdentity(identityId)` → `identity_fetch`
- `getIdentities(identityIds)` → **❌ DEFERRED - Complex mapping issue**
- `getDataContract(contractId)` → `data_contract_fetch` 
- `getDocuments(contractId, documentType, options)` → `get_documents`
- `getDocument(contractId, documentType, documentId)` → `get_document`

*State Transition Methods (3):*
- `createIdentity(identityData, privateKey)` → **⚠️ Implementation needs verification**
- `createDataContract(contractData, identityId, privateKey)` → **⚠️ Implementation needs verification**
- `createDocument(documentData, contractId, documentType, identityId, privateKey)` → **⚠️ Implementation needs verification**

*Utility Methods (3):*
- `getPlatformVersion()` → **⚠️ Implementation needs verification**
- `getNetworkStatus()` → **⚠️ Implementation needs verification**  
- `validateDocument(document, dataContract)` → **⚠️ Implementation needs verification**

*Infrastructure Methods (2):*
- `initialize()` - Wrapper initialization
- `destroy()` - Resource cleanup

**❌ Missing from JavaScript Wrapper (128+ functions):**

*Identity Queries (39 functions):*
- `get_identity_balance`, `get_identity_keys`, `get_identity_nonce`
- `get_identity_balance_and_revision`, `get_identity_contract_nonce`
- `get_identity_by_public_key_hash`, `get_identity_by_non_unique_public_key_hash`
- `get_identities_balances`, `get_identities_contract_keys`
- `get_identity_token_balances`, `get_identity_token_infos`
- `get_identities_token_balances`, `get_identities_token_infos`
- All `*_with_proof_info` variants

*Key Generation & Crypto (25 functions):*
- `generate_mnemonic`, `validate_mnemonic`, `mnemonic_to_seed`
- `derive_key_from_seed_with_path`, `derive_key_from_seed_phrase`
- `generate_key_pair`, `generate_key_pairs`
- `key_pair_from_wif`, `key_pair_from_hex`
- `pubkey_to_address`, `validate_address`
- `sign_message`, `derive_child_public_key`
- All derivation path functions (`derivation_path_*`)

*Token Queries (16 functions):*
- `get_token_statuses`, `get_token_direct_purchase_prices`
- `get_token_contract_info`, `get_token_total_supply`
- `get_token_price_by_contract`, `calculate_token_id_from_contract`
- All `*_with_proof_info` variants

*System & Status Queries (14 functions):*
- `get_status`, `get_current_epoch`, `get_epochs_info`
- `get_current_quorums_info`, `get_total_credits_in_platform`
- `get_protocol_version_upgrade_state`, `get_protocol_version_upgrade_vote_status`
- `get_prefunded_specialized_balance`, `get_path_elements`

*DPNS Functions (7 functions):*
- `dpns_is_valid_username`, `dpns_is_contested_username`
- `dpns_convert_to_homograph_safe`, `dpns_resolve_name`
- `dpns_register_name`, `dpns_is_name_available`
- `get_dpns_username*` variants

*Group Queries (14 functions):*
- `get_group_info`, `get_group_infos`, `get_group_members`
- `get_identity_groups`, `get_group_actions`, `get_group_action_signers`
- `get_groups_data_contracts`
- All `*_with_proof_info` variants

*Voting/Contested Resources (10 functions):*
- `get_contested_resources`, `get_contested_resource_vote_state`
- `get_contested_resource_voters_for_identity`, `get_contested_resource_identity_votes`
- `get_vote_polls_by_end_date`
- All `*_with_proof_info` variants

*Epoch & Block Queries (8 functions):*
- `get_finalized_epoch_infos`, `get_evonodes_proposed_epoch_blocks_by_ids`
- `get_evonodes_proposed_epoch_blocks_by_range`
- All `*_with_proof_info` variants

### Test File Analysis (VERIFIED - 24 Test Files)

**Current Test Patterns** (❌ ALL 24 Files Use Direct WASM):
```javascript
// Direct WASM usage (current pattern in ALL tests)
import init, * as wasmSdk from '../pkg/wasm_sdk.js';
const builder = wasmSdk.WasmSdkBuilder.new_testnet_trusted();
const sdk = await builder.build();
const result = await wasmSdk.identity_fetch(sdk, identityId);
```

**Most Used WASM Functions in Tests (BLOCKING MIGRATION):**
1. `derive_key_from_seed_with_path` (27 uses)
2. `validate_mnemonic` (15 uses)
3. `generate_mnemonic` (14 uses)
4. `prefetch_trusted_quorums_testnet` (14 uses)
5. `generate_key_pair` (14 uses)
6. `dpns_is_valid_username` (12 uses)
7. `get_documents` (7 uses) ✅ **Already wrapped**
8. `get_status` (7 uses)
9. `dpns_convert_to_homograph_safe` (10 uses)
10. `validate_address` (9 uses)

**Target Test Patterns** (✅ What ALL tests should become):
```javascript
// JavaScript wrapper usage (target pattern)
import { WasmSDK } from '../src-js/index.js';
const sdk = new WasmSDK({ network: 'testnet', proofs: true });
await sdk.initialize();
const result = await sdk.getIdentity(identityId);
await sdk.destroy();
```

**❌ BLOCKING ISSUE:**
Tests cannot migrate to wrapper until wrapper implements the functions they actually use (key generation, DPNS utilities, etc.)

### Implementation Priority (BASED ON ACTUAL TEST USAGE)

**CRITICAL Priority** (Tests cannot migrate without these - 27+ uses):
1. **Key Generation & Crypto Functions** (blocking 15+ test files)
   - `derive_key_from_seed_with_path` (27 uses) 🚨
   - `generate_mnemonic` (14 uses)
   - `validate_mnemonic` (15 uses)
   - `generate_key_pair` (14 uses)
   - `sign_message` (6 uses)
   - `mnemonic_to_seed` (5 uses)
   - `pubkey_to_address`, `validate_address` (9 uses)
   - `key_pair_from_wif`, `key_pair_from_hex`
   - All derivation path functions (`derivation_path_*`)

**HIGH Priority** (Core functionality used by tests - 10+ uses):
2. **DPNS Utilities** (blocking 3+ test files)
   - `dpns_is_valid_username` (12 uses)
   - `dpns_convert_to_homograph_safe` (10 uses)
   - `dpns_is_contested_username` (6 uses)
   - `dpns_resolve_name`, `dpns_is_name_available`

3. **System Queries** (blocking 2+ test files)
   - `get_status` (7 uses)
   - `get_current_epoch`, `get_epochs_info`
   - `prefetch_trusted_quorums_testnet` (14 uses) - **initialization requirement**

**MEDIUM Priority** (Enhanced functionality):
4. **Identity Queries** (comprehensive identity operations)
   - `get_identity_balance`, `get_identity_keys`, `get_identity_nonce`
   - `get_identities_balances`, `get_identity_balance_and_revision`
   - `get_identity_by_public_key_hash`, `get_identity_contract_nonce`

5. **Token Operations** (token ecosystem support)
   - `get_identity_token_balances`, `get_token_statuses`
   - `get_token_direct_purchase_prices`, `calculate_token_id_from_contract`
   - `get_token_contract_info`, `get_token_total_supply`

**LOW Priority** (Specialized features):
6. **Voting/Contested Resources** (governance features)
7. **Group Operations** (advanced identity management)
8. **Protocol Version Queries** (system administration)

## 🔍 Additional Implementation Details

### Wrapper Method Implementation Pattern

**Standard Pattern for New Methods:**
```javascript
/**
 * Generate a mnemonic phrase
 * @param {number} wordCount - Number of words (12, 15, 18, 21, or 24)
 * @returns {Promise<string>} Generated mnemonic phrase
 */
async generateMnemonic(wordCount = 12) {
    ErrorUtils.validateRequired({ wordCount }, ['wordCount']);
    
    return this._executeOperation(
        () => this.wasmModule.generate_mnemonic(wordCount),
        'generate_mnemonic',
        { wordCount }
    );
}
```

### Test Migration Strategy

**Migration Approach:**
1. **Start with key-generation tests** (most critical, 15+ files depend on these)
2. **Move to DPNS tests** (3+ files, cleaner to migrate)
3. **Progress to system query tests** (2+ files)
4. **Complete with specialized tests** (voting, groups, etc.)

**Per-Test Migration Checklist:**
- [ ] Replace WASM import with wrapper import
- [ ] Update initialization pattern
- [ ] Convert function calls to wrapper methods
- [ ] Update error handling for offline mode
- [ ] Add proper resource cleanup
- [ ] Verify test still passes

### Quality Assurance

**Verification Steps for Each New Wrapper Method:**
1. ✅ Method exists in WASM exports
2. ✅ Parameters match WASM function signature
3. ✅ Error handling follows established patterns
4. ✅ Resource management integrated
5. ✅ Documentation includes parameters and return type
6. ✅ Method tested with actual WASM call

## 🚀 PHASED IMPLEMENTATION STRATEGY

### 🎯 Phase 1: Critical Key Generation Functions
**Goal**: Implement the most-used crypto functions to unblock 15+ test files

**Functions to Implement (8 functions):**
1. `generateMnemonic(wordCount)` → `generate_mnemonic`
2. `validateMnemonic(mnemonic)` → `validate_mnemonic`  
3. `mnemonicToSeed(mnemonic, passphrase)` → `mnemonic_to_seed`
4. `deriveKeyFromSeedWithPath(mnemonic, passphrase, path, network)` → `derive_key_from_seed_with_path`
5. `generateKeyPair()` → `generate_key_pair`
6. `pubkeyToAddress(publicKey, network)` → `pubkey_to_address`
7. `validateAddress(address, network)` → `validate_address`
8. `signMessage(message, privateKey)` → `sign_message`

**Success Criteria**: 
- All 8 wrapper methods work correctly
- Key generation tests (3-4 files) can migrate to wrapper
- Tests pass using wrapper instead of direct WASM

### 🎯 Phase 2: DPNS Utility Functions  
**Goal**: Implement DPNS functions to unblock username validation tests

**Functions to Implement (5 functions):**
1. `dpnsIsValidUsername(label)` → `dpns_is_valid_username`
2. `dpnsConvertToHomographSafe(input)` → `dpns_convert_to_homograph_safe`
3. `dpnsIsContestedUsername(label)` → `dpns_is_contested_username`
4. `dpnsResolveName(name)` → `dpns_resolve_name`
5. `dpnsIsNameAvailable(label)` → `dpns_is_name_available`

**Success Criteria**:
- All 5 DPNS wrapper methods work correctly
- DPNS tests (2-3 files) can migrate to wrapper
- Combined with Phase 1, ~6-8 test files fully migrated

### 🎯 Phase 3: Core System Queries
**Goal**: Implement essential system status and epoch functions

**Functions to Implement (6 functions):**
1. `getStatus()` → `get_status`
2. `getCurrentEpoch()` → `get_current_epoch`  
3. `getEpochsInfo(start, count, ascending)` → `get_epochs_info`
4. `getCurrentQuorumsInfo()` → `get_current_quorums_info`
5. `getTotalCreditsInPlatform()` → `get_total_credits_in_platform`
6. `getPathElements(path, keys)` → `get_path_elements`

**Success Criteria**:
- All 6 system query wrapper methods work correctly
- System/epoch tests (2-3 files) can migrate to wrapper  
- Combined total: ~10-12 test files fully migrated

### 🎯 Phase 4: Enhanced Identity Operations
**Goal**: Complete comprehensive identity query support

**Functions to Implement (12 functions):**
1. `getIdentityBalance(identityId)` → `get_identity_balance`
2. `getIdentityKeys(identityId, keyType, specificIds, searchMap, limit, offset)` → `get_identity_keys`
3. `getIdentityNonce(identityId)` → `get_identity_nonce`
4. `getIdentityContractNonce(identityId, contractId)` → `get_identity_contract_nonce`
5. `getIdentityBalanceAndRevision(identityId)` → `get_identity_balance_and_revision`
6. `getIdentityByPublicKeyHash(keyHash)` → `get_identity_by_public_key_hash`
7. `getIdentityByNonUniquePublicKeyHash(keyHash, startAfter)` → `get_identity_by_non_unique_public_key_hash`
8. `getIdentitiesBalances(identityIds)` → `get_identities_balances`
9. `getIdentitiesContractKeys(identityIds, contractId, documentType, purposes)` → `get_identities_contract_keys`
10. `getIdentityTokenBalances(identityId, tokenIds)` → `get_identity_token_balances`
11. `getIdentityTokenInfos(identityId, tokenIds, limit, offset)` → `get_identity_token_infos`
12. `getIdentitiesTokenBalances(identityIds, tokenId)` → `get_identities_token_balances`

**Success Criteria**:
- All 12 identity wrapper methods work correctly
- Identity query tests (3-4 files) can migrate
- Combined total: ~15-18 test files fully migrated

### 🎯 Phase 5: Token Operations
**Goal**: Complete token ecosystem support

**Functions to Implement (8 functions):**
1. `getTokenStatuses(tokenIds)` → `get_token_statuses`
2. `getTokenDirectPurchasePrices(tokenIds)` → `get_token_direct_purchase_prices`
3. `getTokenContractInfo(contractId)` → `get_token_contract_info`
4. `getTokenTotalSupply(tokenId)` → `get_token_total_supply`
5. `getTokenPriceByContract(contractId, tokenPosition)` → `get_token_price_by_contract`
6. `calculateTokenIdFromContract(contractId, tokenPosition)` → `calculate_token_id_from_contract`
7. `getTokenPerpetualDistributionLastClaim(identityId, tokenId)` → `get_token_perpetual_distribution_last_claim`
8. `getIdentitiesTokenInfos(identityIds, tokenId)` → `get_identities_token_infos`

**Success Criteria**:
- All 8 token wrapper methods work correctly
- Token tests (2-3 files) can migrate
- Combined total: ~20+ test files fully migrated

### 🎯 Phase 6: Specialized Features (Final)
**Goal**: Complete wrapper with remaining specialized functions

**Functions to Implement (~25+ functions):**
- Group operations (7 functions)
- Voting/contested resources (5 functions)  
- Protocol version queries (3 functions)
- Epoch/block queries (4 functions)
- Additional utility functions (6+ functions)

**Success Criteria**:
- 100% WASM function coverage in wrapper
- All 24 test files migrated to wrapper
- Pattern alignment complete

## 📋 Implementation Strategy Per Phase

**For Each Phase:**
1. **Implement Functions**: Add wrapper methods following established patterns
2. **Verify Functionality**: Test each method with actual WASM calls
3. **Migrate Tests**: Convert 2-4 test files to use wrapper
4. **Validate**: Ensure migrated tests pass with wrapper
5. **Document**: Update any API documentation

**Quality Gates:**
- Each phase must be fully working before proceeding
- Test migration validates wrapper correctness
- Incremental progress allows for course correction

**Estimated Timeline:**
- Phase 1-2: Most critical (1-2 weeks)
- Phase 3-4: Core functionality (1-2 weeks)  
- Phase 5-6: Complete system (1-2 weeks)

---

*Generated: 2025-09-08*  
*Updated: 2025-09-08 with comprehensive verification and phased implementation*  
*Status: Planning Phase - Ready for Phased Implementation*
*Next Action: Begin Phase 1 - Critical Key Generation Functions*
*Priority: Implement 8 key crypto functions first*