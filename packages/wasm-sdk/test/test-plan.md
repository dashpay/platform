# WASM SDK Comprehensive Test Plan

## Test Structure

### 1. SDK Initialization Tests (`sdk-init.test.mjs`)
- WasmSdkBuilder.new_mainnet()
- WasmSdkBuilder.new_mainnet_trusted()
- WasmSdkBuilder.new_testnet()
- WasmSdkBuilder.new_testnet_trusted()
- with_version()
- with_settings()
- build()
- getLatestVersionNumber()

### 2. Wallet/Key Generation Tests (`key-generation.test.mjs`)
- generate_mnemonic (12, 15, 18, 21, 24 words, multiple languages)
- validate_mnemonic
- mnemonic_to_seed
- derive_key_from_seed_phrase
- derive_key_from_seed_with_path
- derivation_path_bip44_mainnet/testnet
- derivation_path_dip9_mainnet/testnet
- derivation_path_dip13_mainnet/testnet
- derive_child_public_key
- xprv_to_xpub
- generate_key_pair
- generate_key_pairs
- key_pair_from_wif
- key_pair_from_hex
- pubkey_to_address
- validate_address
- sign_message

### 3. DPNS Tests (`dpns.test.mjs`)
- dpns_convert_to_homograph_safe
- dpns_is_valid_username
- dpns_is_contested_username
- dpns_register_name
- dpns_is_name_available
- dpns_resolve_name
- get_dpns_username_by_name
- get_dpns_usernames
- get_dpns_username

### 4. Identity Query Tests (`identity-queries.test.mjs`)
- identity_fetch
- identity_fetch_unproved
- get_identity_keys
- get_identity_nonce
- get_identity_contract_nonce
- get_identity_balance
- get_identities_balances
- get_identity_balance_and_revision
- get_identity_by_public_key_hash
- get_identity_by_non_unique_public_key_hash
- get_identities_contract_keys
- get_identity_token_balances

### 5. Document Query Tests (`document-queries.test.mjs`)
- get_documents
- get_document

### 6. Data Contract Query Tests (`contract-queries.test.mjs`)
- data_contract_fetch
- get_data_contract_history
- get_data_contracts

### 7. Token Query Tests (`token-queries.test.mjs`)
- get_identities_token_balances
- get_identity_token_infos
- get_identities_token_infos
- get_token_statuses
- get_token_direct_purchase_prices
- get_token_contract_info
- get_token_perpetual_distribution_last_claim
- get_token_total_supply

### 8. Epoch Query Tests (`epoch-queries.test.mjs`)
- get_epochs_info
- get_finalized_epoch_infos
- get_current_epoch
- get_evonodes_proposed_epoch_blocks_by_ids
- get_evonodes_proposed_epoch_blocks_by_range

### 9. Protocol/System Query Tests (`system-queries.test.mjs`)
- get_protocol_version_upgrade_state
- get_protocol_version_upgrade_vote_status
- get_status
- get_current_quorums_info
- get_total_credits_in_platform
- get_prefunded_specialized_balance
- get_path_elements

### 10. Voting/Contested Resource Tests (`voting-queries.test.mjs`)
- get_contested_resources
- get_contested_resource_vote_state
- get_contested_resource_voters_for_identity
- get_contested_resource_identity_votes
- get_vote_polls_by_end_date

### 11. Group Query Tests (`group-queries.test.mjs`)
- get_group_info
- get_group_infos
- get_group_members
- get_identity_groups
- get_group_actions
- get_group_action_signers
- get_groups_data_contracts

### 12. State Transition Tests (`state-transitions.test.mjs`)
- Token operations:
  - tokenMint
  - tokenBurn
  - tokenTransfer
  - tokenFreeze
  - tokenUnfreeze
  - tokenDestroyFrozen
- Contract operations:
  - contractCreate
  - contractUpdate
- Document operations:
  - documentCreate
  - documentReplace
  - documentDelete
  - documentTransfer
  - documentPurchase
  - documentSetPrice
- Identity operations:
  - identityCreditTransfer
  - identityCreditWithdrawal
  - identityUpdate
- Voting:
  - masternodeVote

### 13. Verification Tests (`verification.test.mjs`)
- verify_identity_response
- verify_data_contract
- verify_documents

### 14. Utility Tests (`utilities.test.mjs`)
- wait_for_state_transition_result
- prefetch_trusted_quorums_mainnet
- prefetch_trusted_quorums_testnet
- testSerialization

## Test Categories

### Expected to Pass
- All wallet/key generation functions
- Basic utility functions
- DPNS validation functions

### Expected to Fail (Need SDK Connection)
- All query functions (require network connection)
- All state transition functions (require funded identities)
- Verification functions (require proof data)

### Test Data Requirements
- Valid identity IDs for testnet
- Valid data contract IDs
- Test private keys
- Test document IDs
- Test token contract information