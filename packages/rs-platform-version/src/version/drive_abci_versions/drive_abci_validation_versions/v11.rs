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
/// - the data trigger `bindings` 3, which refuse a document transition's replace or delete of a
///   token market contract order (`ask` and `bid`): only the token marketplace transitions write
///   them.
/// - the batch `advanced_structure` 2: v1, then the structure checks of token actions protocol
///   v15 introduces, which read only the contract the action carries and so run in check_tx too.
///   A `TokenConfigUpdate` can make a token tradeable only if it is transferable.
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
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                advanced_structure: 2, // a tradeable token must be transferable, checked in check_tx too
                data_triggers: DriveAbciValidationDataTriggerAndBindingVersions {
                    bindings: 3, // the token market contract's orders are written by the marketplace transitions only
                    triggers: DriveAbciValidationDataTriggerVersions {
                        create_domain_data_trigger: 2,
                        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                            .state_transitions
                            .batch_state_transition
                            .data_triggers
                            .triggers
                    },
                },
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
