//! Compile-time guards for SDK constructors that hard-code versioned
//! basic-structure checks.
//!
//! Several SDK constructors call a concrete structural validation directly
//! (search for `LOCKSTEP` comments in this directory). If the underlying
//! server basic_structure ever bumps to a higher version without the SDK
//! constructor also being updated, those constructors would silently keep
//! running the older check and could broadcast transitions the network
//! rejects.

use platform_version::version::LATEST_PLATFORM_VERSION;

macro_rules! const_assert_matches {
    ($expr:expr, $pattern:pat) => {
        const _: [(); 1] = [(); matches!($expr, $pattern) as usize];
    };
}

const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_create_state_transition
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_create_from_addresses_state_transition
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_top_up_from_addresses_state_transition
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_credit_transfer_state_transition
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_credit_transfer_to_addresses_state_transition
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .address_credit_withdrawal
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .address_funds_from_asset_lock
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .address_funds_transfer
        .basic_structure,
    Some(0)
);
const_assert_matches!(
    LATEST_PLATFORM_VERSION
        .drive_abci
        .validation_and_processing
        .state_transitions
        .identity_credit_withdrawal_state_transition
        .basic_structure,
    Some(1)
);
const_assert_matches!(
    (
        LATEST_PLATFORM_VERSION
            .dpp
            .state_transitions
            .identities
            .identity_update
            .basic_structure,
        LATEST_PLATFORM_VERSION
            .drive_abci
            .validation_and_processing
            .state_transitions
            .legacy_identity_update_state_transition
            .basic_structure
    ),
    (Some(0), Some(0))
);

#[cfg(test)]
mod tests {
    use super::LATEST_PLATFORM_VERSION;

    /// Constructors with `LOCKSTEP` notes that hard-code the v0 basic-structure
    /// check. Each entry is `(label, basic_structure_field_value)`.
    fn sdk_v0_lockstep_dispatch_fields() -> Vec<(&'static str, Option<u16>)> {
        let v = LATEST_PLATFORM_VERSION;
        let st = &v.drive_abci.validation_and_processing.state_transitions;
        vec![
            (
                "identity_create_state_transition",
                st.identity_create_state_transition.basic_structure,
            ),
            (
                "identity_create_from_addresses_state_transition",
                st.identity_create_from_addresses_state_transition
                    .basic_structure,
            ),
            (
                "identity_top_up_from_addresses_state_transition",
                st.identity_top_up_from_addresses_state_transition
                    .basic_structure,
            ),
            (
                "identity_credit_transfer_state_transition",
                st.identity_credit_transfer_state_transition.basic_structure,
            ),
            (
                "identity_credit_transfer_to_addresses_state_transition",
                st.identity_credit_transfer_to_addresses_state_transition
                    .basic_structure,
            ),
            (
                "address_credit_withdrawal",
                st.address_credit_withdrawal.basic_structure,
            ),
            (
                "address_funds_from_asset_lock",
                st.address_funds_from_asset_lock.basic_structure,
            ),
            (
                "address_funds_transfer",
                st.address_funds_transfer.basic_structure,
            ),
        ]
    }

    #[test]
    fn sdk_constructors_hardcoded_v0_dispatch_still_matches_latest_platform_version() {
        let mismatches: Vec<(&'static str, Option<u16>)> = sdk_v0_lockstep_dispatch_fields()
            .into_iter()
            .filter(|(_, version)| *version != Some(0))
            .collect();

        assert!(
            mismatches.is_empty(),
            "SDK constructor(s) hard-code the v0 basic-structure check but the \
             latest PlatformVersion no longer resolves their drive-abci \
             basic_structure to Some(0): {:?}. Update the constructor(s) \
             (search for `LOCKSTEP` in packages/rs-dpp/src/state_transition) \
             so they dispatch to the new version, or migrate them to a \
             versioned wrapper as IdentityUpdateTransitionV0 does.",
            mismatches
        );
    }

    #[test]
    fn identity_credit_withdrawal_v1_constructor_dispatch_still_matches_latest_platform_version() {
        let actual = LATEST_PLATFORM_VERSION
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_credit_withdrawal_state_transition
            .basic_structure;
        assert_eq!(
            actual,
            Some(1),
            "drive-abci identity_credit_withdrawal_state_transition.basic_structure \
             changed from Some(1); the SDK constructor for \
             IdentityCreditWithdrawalTransitionV1 hard-codes the v1 check via \
             IdentityCreditWithdrawalTransition::basic_structure_rules_v1 and \
             must be updated to dispatch to the new version before bumping this."
        );
    }

    #[test]
    fn identity_update_dpp_and_legacy_basic_structure_move_together() {
        let v = LATEST_PLATFORM_VERSION;
        let dpp_field = v
            .dpp
            .state_transitions
            .identities
            .identity_update
            .basic_structure;
        let legacy_drive_abci_field = v
            .drive_abci
            .validation_and_processing
            .state_transitions
            .legacy_identity_update_state_transition
            .basic_structure;
        assert_eq!(
            dpp_field,
            Some(0),
            "DPP-owned identity_update.basic_structure changed from Some(0); \
             update both DPP `IdentityUpdateTransitionV0::validate_basic_structure` \
             and drive-abci's identity_update dispatcher to handle the new version."
        );
        assert_eq!(
            dpp_field, legacy_drive_abci_field,
            "DPP-owned identity_update.basic_structure ({:?}) diverged from the \
             legacy drive-abci field legacy_identity_update_state_transition.basic_structure \
             ({:?}). These two fields must move together until the legacy field is \
             removed/migrated; bumping only one will produce silent client/server \
             drift.",
            dpp_field, legacy_drive_abci_field
        );
    }
}
