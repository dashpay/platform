// Sinon-based stub for '@dashevo/wasm-sdk' used by unit tests
// Lightweight stub for '@dashevo/wasm-sdk' used by Karma unit tests
// Returns structured records and tracks calls for verification without external libs.

const calls = [];
const record = (name, args) => ({ __stub: true, called: name, args: Array.from(args) });

export default async function initWasmSdk() { return undefined; }

export function __getCalls() { return calls.slice(); }
export function __clearCalls() { calls.length = 0; }

// Minimal builder used by EvoSDK.connect()
export class WasmSdkBuilder {
  static new_testnet() { return new WasmSdkBuilder('testnet', false); }
  static new_mainnet() { return new WasmSdkBuilder('mainnet', false); }
  static new_testnet_trusted() { return new WasmSdkBuilder('testnet', true); }
  static new_mainnet_trusted() { return new WasmSdkBuilder('mainnet', true); }
  constructor(network, trusted) { this.network = network; this.trusted = trusted; }
  with_version() { return this; }
  with_proofs() { return this; }
  with_settings() { return this; }
  build() { return new WasmSdk(); }
}

export class WasmSdk {}

// Documents
export function get_documents(...args) { const r = record('get_documents', args); calls.push(r); return Promise.resolve(r); }
export function get_documents_with_proof_info(...args) { const r = record('get_documents_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_document(...args) { const r = record('get_document', args); calls.push(r); return Promise.resolve(r); }
export function get_document_with_proof_info(...args) { const r = record('get_document_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Identities
export function identity_fetch(...args) { const r = record('identity_fetch', args); calls.push(r); return Promise.resolve(r); }
export function identity_fetch_with_proof_info(...args) { const r = record('identity_fetch_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function identity_fetch_unproved(...args) { const r = record('identity_fetch_unproved', args); calls.push(r); return Promise.resolve(r); }
export function get_identity_keys(...args) { const r = record('get_identity_keys', args); calls.push(r); return Promise.resolve(r); }

// Contracts
export function data_contract_fetch(...args) { const r = record('data_contract_fetch', args); calls.push(r); return Promise.resolve(r); }
export function data_contract_fetch_with_proof_info(...args) { const r = record('data_contract_fetch_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_data_contract_history(...args) { const r = record('get_data_contract_history', args); calls.push(r); return Promise.resolve(r); }
export function get_data_contract_history_with_proof_info(...args) { const r = record('get_data_contract_history_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_data_contracts(...args) { const r = record('get_data_contracts', args); calls.push(r); return Promise.resolve(r); }
export function get_data_contracts_with_proof_info(...args) { const r = record('get_data_contracts_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Tokens (queries)
export function get_token_price_by_contract(...args) { const r = record('get_token_price_by_contract', args); calls.push(r); return Promise.resolve(r); }
export function get_token_total_supply(...args) { const r = record('get_token_total_supply', args); calls.push(r); return Promise.resolve(r); }
export function get_token_total_supply_with_proof_info(...args) { const r = record('get_token_total_supply_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_token_statuses(...args) { const r = record('get_token_statuses', args); calls.push(r); return Promise.resolve(r); }
export function get_token_statuses_with_proof_info(...args) { const r = record('get_token_statuses_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_identities_token_balances(...args) { const r = record('get_identities_token_balances', args); calls.push(r); return Promise.resolve(r); }
export function get_identities_token_balances_with_proof_info(...args) { const r = record('get_identities_token_balances_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_identity_token_infos(...args) { const r = record('get_identity_token_infos', args); calls.push(r); return Promise.resolve(r); }
export function get_identities_token_infos(...args) { const r = record('get_identities_token_infos', args); calls.push(r); return Promise.resolve(r); }
export function get_identity_token_infos_with_proof_info(...args) { const r = record('get_identity_token_infos_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_identities_token_infos_with_proof_info(...args) { const r = record('get_identities_token_infos_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_token_direct_purchase_prices(...args) { const r = record('get_token_direct_purchase_prices', args); calls.push(r); return Promise.resolve(r); }
export function get_token_direct_purchase_prices_with_proof_info(...args) { const r = record('get_token_direct_purchase_prices_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_token_contract_info(...args) { const r = record('get_token_contract_info', args); calls.push(r); return Promise.resolve(r); }
export function get_token_contract_info_with_proof_info(...args) { const r = record('get_token_contract_info_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_token_perpetual_distribution_last_claim(...args) { const r = record('get_token_perpetual_distribution_last_claim', args); calls.push(r); return Promise.resolve(r); }
export function get_token_perpetual_distribution_last_claim_with_proof_info(...args) { const r = record('get_token_perpetual_distribution_last_claim_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// DPNS helpers (pure)
export function dpns_convert_to_homograph_safe(input) { return String(input).toLowerCase(); }
export function dpns_is_valid_username(label) { return /^[a-z0-9-_]{3,63}$/.test(String(label)); }
export function dpns_is_contested_username(label) { return false; }
export function dpns_is_name_available(...args) { const r = record('dpns_is_name_available', args); calls.push(r); return Promise.resolve(true); }
export function dpns_resolve_name(...args) { const r = record('dpns_resolve_name', args); calls.push(r); return Promise.resolve(r); }
export function dpns_register_name(...args) { const r = record('dpns_register_name', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_usernames(...args) { const r = record('get_dpns_usernames', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_username(...args) { const r = record('get_dpns_username', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_usernames_with_proof_info(...args) { const r = record('get_dpns_usernames_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_username_with_proof_info(...args) { const r = record('get_dpns_username_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_username_by_name(...args) { const r = record('get_dpns_username_by_name', args); calls.push(r); return Promise.resolve(r); }
export function get_dpns_username_by_name_with_proof_info(...args) { const r = record('get_dpns_username_by_name_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Epoch
export function get_epochs_info(...args) { const r = record('get_epochs_info', args); calls.push(r); return Promise.resolve(r); }
export function get_epochs_info_with_proof_info(...args) { const r = record('get_epochs_info_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_finalized_epoch_infos(...args) { const r = record('get_finalized_epoch_infos', args); calls.push(r); return Promise.resolve(r); }
export function get_finalized_epoch_infos_with_proof_info(...args) { const r = record('get_finalized_epoch_infos_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_current_epoch(...args) { const r = record('get_current_epoch', args); calls.push(r); return Promise.resolve(r); }
export function get_current_epoch_with_proof_info(...args) { const r = record('get_current_epoch_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_evonodes_proposed_epoch_blocks_by_ids(...args) { const r = record('get_evonodes_proposed_epoch_blocks_by_ids', args); calls.push(r); return Promise.resolve(r); }
export function get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(...args) { const r = record('get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_evonodes_proposed_epoch_blocks_by_range(...args) { const r = record('get_evonodes_proposed_epoch_blocks_by_range', args); calls.push(r); return Promise.resolve(r); }
export function get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(...args) { const r = record('get_evonodes_proposed_epoch_blocks_by_range_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Protocol
export function get_protocol_version_upgrade_state(...args) { const r = record('get_protocol_version_upgrade_state', args); calls.push(r); return Promise.resolve(r); }
export function get_protocol_version_upgrade_state_with_proof_info(...args) { const r = record('get_protocol_version_upgrade_state_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_protocol_version_upgrade_vote_status(...args) { const r = record('get_protocol_version_upgrade_vote_status', args); calls.push(r); return Promise.resolve(r); }
export function get_protocol_version_upgrade_vote_status_with_proof_info(...args) { const r = record('get_protocol_version_upgrade_vote_status_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// System
export function get_status(...args) { const r = record('get_status', args); calls.push(r); return Promise.resolve(r); }
export function get_current_quorums_info(...args) { const r = record('get_current_quorums_info', args); calls.push(r); return Promise.resolve(r); }
export function get_total_credits_in_platform(...args) { const r = record('get_total_credits_in_platform', args); calls.push(r); return Promise.resolve(r); }
export function get_total_credits_in_platform_with_proof_info(...args) { const r = record('get_total_credits_in_platform_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_prefunded_specialized_balance(...args) { const r = record('get_prefunded_specialized_balance', args); calls.push(r); return Promise.resolve(r); }
export function get_prefunded_specialized_balance_with_proof_info(...args) { const r = record('get_prefunded_specialized_balance_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function wait_for_state_transition_result(...args) { const r = record('wait_for_state_transition_result', args); calls.push(r); return Promise.resolve(r); }
export function get_path_elements(...args) { const r = record('get_path_elements', args); calls.push(r); return Promise.resolve(r); }
export function get_path_elements_with_proof_info(...args) { const r = record('get_path_elements_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Group
export function get_contested_resources(...args) { const r = record('get_contested_resources', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resources_with_proof_info(...args) { const r = record('get_contested_resources_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resource_voters_for_identity(...args) { const r = record('get_contested_resource_voters_for_identity', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resource_voters_for_identity_with_proof_info(...args) { const r = record('get_contested_resource_voters_for_identity_with_proof_info', args); calls.push(r); return Promise.resolve(r); }

// Voting (queries)
export function get_contested_resource_vote_state(...args) { const r = record('get_contested_resource_vote_state', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resource_vote_state_with_proof_info(...args) { const r = record('get_contested_resource_vote_state_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resource_identity_votes(...args) { const r = record('get_contested_resource_identity_votes', args); calls.push(r); return Promise.resolve(r); }
export function get_contested_resource_identity_votes_with_proof_info(...args) { const r = record('get_contested_resource_identity_votes_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
export function get_vote_polls_by_end_date(...args) { const r = record('get_vote_polls_by_end_date', args); calls.push(r); return Promise.resolve(r); }
export function get_vote_polls_by_end_date_with_proof_info(...args) { const r = record('get_vote_polls_by_end_date_with_proof_info', args); calls.push(r); return Promise.resolve(r); }
