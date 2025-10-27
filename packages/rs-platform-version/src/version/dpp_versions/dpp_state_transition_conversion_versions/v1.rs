use crate::version::dpp_versions::dpp_state_transition_conversion_versions::DPPStateTransitionConversionVersions;

pub const STATE_TRANSITION_CONVERSION_VERSIONS_V1: DPPStateTransitionConversionVersions =
    DPPStateTransitionConversionVersions {
        identity_to_identity_create_transition: 0,
        identity_to_identity_top_up_transition: 0,
        identity_to_identity_transfer_transition: 0,
        identity_to_identity_withdrawal_transition: 0,
        identity_to_identity_create_transition_with_signer: 0,
        inputs_to_identity_create_from_addresses_transition_with_signer: 0,
    };
