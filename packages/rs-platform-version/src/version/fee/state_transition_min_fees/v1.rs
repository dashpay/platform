use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;

pub const STATE_TRANSITION_MIN_FEES_VERSION1: StateTransitionMinFees = StateTransitionMinFees {
    credit_transfer: 100000,
    credit_transfer_to_addresses: 500000,
    credit_withdrawal: 400_000_000, // credit withdrawals are more expensive than the rest
    identity_update: 100000,
    document_batch_sub_transition: 100000,
    contract_create: 100000,
    contract_update: 100000,
    masternode_vote: 100000,
    // Address-based state transitions
    address_credit_withdrawal: 400_000_000, // withdrawals are expensive
    address_funds_transfer_input_cost: 500_000,
    address_funds_transfer_output_cost: 6_000_000,
    identity_create_base_cost: 2_000_000,
    identity_key_in_creation_cost: 6_500_000,
    identity_topup_base_cost: 500_000,
    // Expected (display) fee for AddressFundingFromAssetLockTransition —
    // estimate of the GroveDB-metered charge, not a consensus value.
    // Calibrated from apply=true execution (see the drive-abci
    // `expected_fee_calibration` tests) with headroom for tree growth;
    // observed testnet actuals for a 0-input/1-output funding:
    // 14_964_200 and 14_702_160 credits.
    address_funding_expected_base_fee: 10_000_000,
    address_funding_expected_fee_per_input: 2_000_000,
    address_funding_expected_fee_per_output: 7_500_000,
};
