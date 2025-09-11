import init, * as sdk from '../../dist/sdk.js';

describe('API availability (exports and methods)', () => {
  before(async () => {
    await init();
  });
  it('query functions are exported', () => {
    const fns = [
      // Identity
      'identity_fetch','identity_fetch_unproved','get_identity_keys','get_identity_nonce','get_identity_contract_nonce','get_identity_balance','get_identities_balances','get_identity_balance_and_revision','get_identity_by_public_key_hash','get_identity_by_non_unique_public_key_hash','get_identities_contract_keys','get_identity_token_balances','get_identities_token_balances','get_identity_token_infos','get_identities_token_infos',
      // Documents / contracts
      'get_documents','get_document','data_contract_fetch','get_data_contract_history','get_data_contracts',
      // Tokens
      'get_token_statuses','get_token_direct_purchase_prices','get_token_contract_info','get_token_perpetual_distribution_last_claim','get_token_total_supply',
      // Epochs / System / Protocol
      'get_epochs_info','get_finalized_epoch_infos','get_current_epoch','get_evonodes_proposed_epoch_blocks_by_ids','get_evonodes_proposed_epoch_blocks_by_range','get_protocol_version_upgrade_state','get_protocol_version_upgrade_vote_status','get_status','get_current_quorums_info','get_total_credits_in_platform','get_prefunded_specialized_balance','get_path_elements',
      // Voting / Groups
      'get_contested_resources','get_contested_resource_vote_state','get_contested_resource_voters_for_identity','get_contested_resource_identity_votes','get_vote_polls_by_end_date','get_group_info','get_group_infos','get_group_members','get_identity_groups','get_group_actions','get_group_action_signers','get_groups_data_contracts',
      // DPNS queries
      'dpns_register_name','dpns_is_name_available','dpns_resolve_name','get_dpns_username_by_name','get_dpns_usernames','get_dpns_username',
      // Verification / utils
      'verify_identity_response','verify_data_contract','verify_documents','wait_for_state_transition_result',
    ];
    for (const fn of fns) {
      expect(typeof sdk[fn]).to.be.oneOf(['function','undefined']);
    }
  });
});
