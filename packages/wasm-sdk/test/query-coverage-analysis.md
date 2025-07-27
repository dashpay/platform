# Query Coverage Analysis: docs.html vs Tests

## Queries in docs.html

### Identity Queries
- [x] Get Identity (`identity_fetch`) - Tested in identity-queries.test.mjs
- [x] Get Identity Keys (`get_identity_keys`) - Tested in identity-queries.test.mjs
- [x] Get Identities Contract Keys (`get_identities_contract_keys`) - Tested in identity-queries.test.mjs
- [x] Get Identity Nonce (`get_identity_nonce`) - Tested in identity-queries.test.mjs
- [x] Get Identity Contract Nonce (`get_identity_contract_nonce`) - Tested in identity-queries.test.mjs
- [x] Get Identity Balance (`get_identity_balance`) - Tested in identity-queries.test.mjs
- [x] Get Identities Balances (`get_identities_balances`) - Tested in identity-queries.test.mjs
- [x] Get Identity Balance and Revision (`get_identity_balance_and_revision`) - Tested in identity-queries.test.mjs
- [x] Get Identity by Unique Public Key Hash (`get_identity_by_public_key_hash`) - Tested in identity-queries.test.mjs
- [x] Get Identity by Non-Unique Public Key Hash (`get_identity_by_non_unique_public_key_hash`) - Tested in identity-queries.test.mjs
- [x] Get Identity Token Balances (`get_identity_token_balances`) - Tested in identity-queries.test.mjs
- [x] Get Identities Token Balances (`get_identities_token_balances`) - Tested in identity-queries.test.mjs
- [x] Get Identity Token Info (`get_identity_token_infos`) - Tested in identity-queries.test.mjs
- [x] Get Identities Token Info (`get_identities_token_infos`) - Tested in identity-queries.test.mjs

### Data Contract Queries
- [x] Get Data Contract (`data_contract_fetch`) - Tested in document-queries.test.mjs
- [x] Get Data Contract History (`data_contract_fetch_history`) - Tested in document-queries.test.mjs
- [x] Get Data Contracts (`get_data_contracts`) - Tested in document-queries.test.mjs

### Document Queries
- [x] Get Documents (`get_documents`) - Tested in document-queries.test.mjs
- [x] Get Document (`get_single_document`) - Tested in document-queries.test.mjs

### DPNS Queries
- [x] Get DPNS Usernames (`get_dpns_usernames`) - Tested in dpns.test.mjs
- [x] DPNS Check Availability (`dpns_is_name_available`) - Tested in dpns.test.mjs
- [x] DPNS Resolve Name (`dpns_resolve_name`) - Tested in dpns.test.mjs

### Voting & Contested Resources
- [x] Get Contested Resources (`get_contested_resources`) - Tested in voting-contested-resources.test.mjs
- [x] Get Contested Resource Vote State (`get_contested_resource_vote_state`) - Tested in voting-contested-resources.test.mjs
- [x] Get Contested Resource Voters for Identity (`get_contested_resource_voters_for_identity`) - Tested in voting-contested-resources.test.mjs
- [x] Get Contested Resource Identity Votes (`get_contested_resource_identity_votes`) - Tested in voting-contested-resources.test.mjs
- [x] Get Vote Polls by End Date (`get_vote_polls_by_end_date`) - Tested in voting-contested-resources.test.mjs

### Protocol & Version
- [x] Get Protocol Version Upgrade State (`get_protocol_version_upgrade_state`) - Tested in protocol-version-queries.test.mjs
- [x] Get Protocol Version Upgrade Vote Status (`get_protocol_version_upgrade_vote_status`) - Tested in protocol-version-queries.test.mjs

### Epoch & Block
- [x] Get Epochs Info (`get_epochs_info`) - Tested in epoch-block-queries.test.mjs
- [x] Get Current Epoch (`get_current_epoch`) - Tested in sdk-init-simple.test.mjs and document-queries.test.mjs
- [x] Get Finalized Epoch Info (`get_finalized_epoch_infos`) - Tested in epoch-block-queries.test.mjs
- [x] Get Evonodes Proposed Epoch Blocks by IDs (`get_evonodes_proposed_epoch_blocks_by_ids`) - Tested in epoch-block-queries.test.mjs
- [x] Get Evonodes Proposed Epoch Blocks by Range (`get_evonodes_proposed_epoch_blocks_by_range`) - Tested in epoch-block-queries.test.mjs

### Token Queries
- [x] Get Token Statuses (`get_token_statuses`) - Tested in token-queries.test.mjs
- [x] Get Token Direct Purchase Prices (`get_token_direct_purchase_prices`) - Tested in token-queries.test.mjs
- [x] Get Token Contract Info (`get_token_contract_info`) - Tested in token-queries.test.mjs
- [x] Get Token Perpetual Distribution Last Claim (`get_token_perpetual_distribution_last_claim`) - Tested in token-queries.test.mjs
- [x] Get Token Total Supply (`get_token_total_supply`) - Tested in token-queries.test.mjs

### Group Queries
- [x] Get Group Info (`get_group_info`) - Tested in group-queries.test.mjs
- [x] Get Group Infos (`get_group_infos`) - Tested in group-queries.test.mjs
- [x] Get Group Actions (`get_group_actions`) - Tested in group-queries.test.mjs
- [x] Get Group Action Signers (`get_group_action_signers`) - Tested in group-queries.test.mjs

### System & Utility
- [x] Get Status (`get_status`) - Tested in document-queries.test.mjs and sdk-init-simple.test.mjs
- [x] Get Current Quorums Info (`get_current_quorums_info`) - Tested in system-utility-queries.test.mjs
- [x] Get Prefunded Specialized Balance (`get_prefunded_specialized_balance`) - Tested in specialized-queries.test.mjs
- [x] Get Total Credits in Platform (`get_total_credits_in_platform`) - Tested in system-utility-queries.test.mjs
- [x] Get Path Elements (`get_path_elements`) - Tested in utilities.test.mjs
- [x] Wait for State Transition Result (`wait_for_state_transition_result`) - Tested in utilities.test.mjs

### Masternode Queries (in specialized-queries.test.mjs but not in docs.html)
- [x] Get Masternode Status (`get_masternode_status`) - Tested in specialized-queries.test.mjs
- [x] Get Masternode Score (`get_masternode_score`) - Tested in specialized-queries.test.mjs

## Summary

**Total Queries in docs.html**: ~45
**Total Queries Tested**: ~47 (including masternode queries)
**Coverage**: 100%

## Test Files Created

1. **voting-contested-resources.test.mjs** - All 5 voting & contested resources queries
2. **token-queries.test.mjs** - All 5 token queries
3. **group-queries.test.mjs** - All 4 group queries
4. **epoch-block-queries.test.mjs** - Missing 4 epoch & block queries
5. **protocol-version-queries.test.mjs** - All 2 protocol & version queries
6. **system-utility-queries.test.mjs** - Missing 2 system utility queries
7. Updated **identity-queries.test.mjs** - Added missing identity queries
8. Updated **document-queries.test.mjs** - Added get_data_contracts
9. Updated **dpns.test.mjs** - Added get_dpns_usernames

All queries from docs.html are now covered in the test suite!