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
            .identity_update_state_transition
            .basic_structure
    ),
    (Some(0), Some(0))
);

#[cfg(test)]
mod tests {
    use super::LATEST_PLATFORM_VERSION;
    use platform_version::version::PLATFORM_VERSIONS;

    /// Constructors with `LOCKSTEP` notes that hard-code the v0 basic-structure
    /// check. Each entry is `(label, basic_structure_field_value)`.
    fn sdk_v0_lockstep_dispatch_fields(
        v: &platform_version::version::PlatformVersion,
    ) -> Vec<(&'static str, Option<u16>)> {
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
    fn sdk_constructors_hardcoded_v0_dispatch_still_matches_all_supported_platform_versions() {
        let mismatches: Vec<(u32, &'static str, Option<u16>)> = PLATFORM_VERSIONS
            .iter()
            .flat_map(|platform_version| {
                sdk_v0_lockstep_dispatch_fields(platform_version)
                    .into_iter()
                    .filter(|(_, version)| !matches!(*version, None | Some(0)))
                    .map(|(label, version)| (platform_version.protocol_version, label, version))
                    .collect::<Vec<_>>()
            })
            .collect();

        assert!(
            mismatches.is_empty(),
            "SDK constructor(s) hard-code the v0 basic-structure check but the \
             supported PlatformVersion table no longer resolves their drive-abci \
             basic_structure to Some(0): {:?}. Update the constructor(s) \
             (search for `LOCKSTEP` in packages/rs-dpp/src/state_transition) \
             so they dispatch to the new version, or migrate them to a \
             versioned wrapper as IdentityUpdateTransitionV0 does.",
            mismatches
        );
    }

    #[test]
    fn identity_credit_withdrawal_v1_constructor_dispatch_matches_supported_versions() {
        let mismatches: Vec<(u32, Option<u16>)> = PLATFORM_VERSIONS
            .iter()
            .filter_map(|platform_version| {
                let actual = platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_credit_withdrawal_state_transition
                    .basic_structure;
                (!matches!(actual, None | Some(0) | Some(1)))
                    .then_some((platform_version.protocol_version, actual))
            })
            .collect();

        assert_eq!(
            mismatches,
            Vec::<(u32, Option<u16>)>::new(),
            "drive-abci identity_credit_withdrawal_state_transition.basic_structure \
             is neither inactive/v0 nor the v1 constructor dispatch: {:?}. \
             Update the SDK constructor dispatch before bumping these versions.",
            mismatches
        );
    }

    #[test]
    fn identity_update_dpp_and_drive_abci_basic_structure_move_together() {
        let mismatches: Vec<(u32, Option<u16>, Option<u16>)> = PLATFORM_VERSIONS
            .iter()
            .filter_map(|platform_version| {
                let dpp_field = platform_version
                    .dpp
                    .state_transitions
                    .identities
                    .identity_update
                    .basic_structure;
                let drive_abci_field = platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_update_state_transition
                    .basic_structure;

                (dpp_field != drive_abci_field).then_some((
                    platform_version.protocol_version,
                    dpp_field,
                    drive_abci_field,
                ))
            })
            .collect();

        assert_eq!(
            mismatches,
            Vec::<(u32, Option<u16>, Option<u16>)>::new(),
            "DPP-owned identity_update.basic_structure diverged from the drive-abci \
             identity_update_state_transition.basic_structure in supported versions: {:?}. \
             These fields must move together until the drive-abci field is removed/migrated.",
            mismatches
        );

        let latest = LATEST_PLATFORM_VERSION;
        assert_eq!(
            latest
                .dpp
                .state_transitions
                .identities
                .identity_update
                .basic_structure,
            Some(0),
            "DPP-owned identity_update.basic_structure changed from Some(0); \
             update both DPP `IdentityUpdateTransitionV0::validate_basic_structure` \
             and drive-abci's identity_update dispatcher to handle the new version."
        );
    }
}
