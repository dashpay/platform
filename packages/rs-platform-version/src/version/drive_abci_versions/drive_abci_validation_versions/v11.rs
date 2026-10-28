use super::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use super::{
    DriveAbciDocumentsStateTransitionValidationVersions, DriveAbciStateTransitionValidationVersion,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationDataTriggerAndBindingVersions,
    DriveAbciValidationDataTriggerVersions, DriveAbciValidationVersions,
};

/// Drive ABCI validation versions 11, introduced in protocol v15. Differs from v10 in:
///
/// - the identity update `advanced_structure`, 1 rather than 0: an identity update whose added
///   key's proof of possession does not verify (a key whose public key data does not decode fails
///   it too) is refused without a fee and without using the identity's nonce, instead of being
///   charged. The identity's signature does not cover the proofs of possession of the keys it
///   adds. A revision mismatch is still charged.
/// - `create_domain_data_trigger` 2, which keeps only the parent domain checks of a DPNS domain
///   create: the DPNS contract v3 (`SYSTEM_DATA_CONTRACT_VERSIONS_V4`) checks the normalized
///   names, the preorder and the identity record with schema keywords.
/// - the batch `advanced_structure` 2: v1, then the structure checks of token actions protocol
///   v15 introduces, which read only the contract the action carries and so run in check_tx too.
///   A `TokenConfigUpdate` can make a token tradeable only if it is transferable.
/// - the contract update `transform_into_action` 1: v0, then in check_tx and recheck, which run no
///   state validation, the updated contract is held to the compiled size cap with its time fields
///   at their widest, a bound on what a block measures once it restores the stored contract's
///   creation fields. It reads no state, so it covers an update to a contract created earlier in
///   the same block too. Mempool admission only: a block runs v0's state validation.
///
/// v10 remains unchanged for `PROTOCOL_VERSION_14` chain replay.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            identity_update_state_transition: DriveAbciStateTransitionValidationVersion {
                advanced_structure: Some(1), // added key proof of possession failures are refused unpaid
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .identity_update_state_transition
            },
            contract_update_state_transition: DriveAbciStateTransitionValidationVersion {
                transform_into_action: 1, // check_tx holds the update to the compiled size cap
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .contract_update_state_transition
            },
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                advanced_structure: 2, // a tradeable token must be transferable, checked in check_tx too
                data_triggers: DriveAbciValidationDataTriggerAndBindingVersions {
                    triggers: DriveAbciValidationDataTriggerVersions {
                        create_domain_data_trigger: 2,
                        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                            .state_transitions
                            .batch_state_transition
                            .data_triggers
                            .triggers
                    },
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .batch_state_transition
                        .data_triggers
                },
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
