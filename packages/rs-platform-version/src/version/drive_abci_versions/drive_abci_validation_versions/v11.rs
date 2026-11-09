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
/// - the identity create `basic_structure`, 1 rather than 0, and `advanced_structure`, 2 rather
///   than 1: the check that the identity id is the one the asset lock's outpoint derives moves
///   from advanced structure to basic structure, as it reads only the transition. A wrong
///   identity id is still refused without a fee, but before the asset lock proof is verified,
///   so a transition with a wrong identity id and an asset lock that is spent or whose proof
///   does not verify is refused for its identity id. An asset lock proof whose structure is
///   invalid is still refused first, for its structure. Advanced structure 2 still charges the
///   double SHA-256 of deriving the identity id, so every identity create costs what it did.
/// - `create_domain_data_trigger` 2, which keeps only the parent domain checks of a DPNS domain
///   create: the DPNS contract v3 (`SYSTEM_DATA_CONTRACT_VERSIONS_V4`) checks the normalized
///   names, the preorder and the identity record with schema keywords.
/// - the data trigger `bindings` 3: v2 without the DashPay bindings, the contact request create
///   and the profile create and replace triggers, as the DashPay contract v3
///   (`SYSTEM_DATA_CONTRACT_VERSIONS_V4`) declares their checks as schema keywords; and with a
///   refusal of a document transition's replace or delete of a token market contract order
///   (`ask` and `bid`), as only the token marketplace transitions write them.
/// - the batch `advanced_structure` 2: v1, then the structure checks of token actions protocol
///   v15 introduces, which read only the contract the action carries and so run in check_tx too.
///   A `TokenConfigUpdate` can make a token tradeable only if it is transferable.
/// - the masternode vote `transform_into_action`, 1 rather than 0: a vote on a yes/no poll looks up
///   the masternode's previous answer on that poll.
/// - the contract update `transform_into_action` 1: v0, then in check_tx and recheck, which run no
///   state validation, the updated contract is held to the compiled size cap with its time fields
///   at their widest, a bound on what a block measures once it restores the stored contract's
///   creation fields. It reads no state, so it covers an update to a contract created earlier in
///   the same block too. Mempool admission only: a block runs v0's state validation.
/// - document create `structure_validation` 2: v1, then a create of the decision proposals
///   contract's `proposal` (protocol version 15) must state exactly the decision proposal fund
///   under the proposal's name, its rule must make a yes/no poll within the system limits, and
///   it must end one to four weeks after the block. The create conversion 1 opens and funds its
///   poll.
/// - document create `state_validation` 3: v2, then a create outside a contest, of a type whose
///   contest reads more than its index values (`fieldMatches` on a property outside the index,
///   which registration refuses from protocol version 15), is refused
///   (`DuplicateUniqueIndexError`) while a contest for the same index values is live: the
///   document would take the unique index entry the contest awards. Only such a type pays the
///   lookup, so a DPNS domain create costs what it did.
/// - `document_reference_validation` 1: v0, but the `refersTo` lookup of a reference declaring
///   `consume` reads the commitment it finds with its stored element's storage flags
///   (`query_documents_with_flags`) and keeps that read in the `ConsumedDocument`, so the create
///   conversion 1 deletes the commitment from it without reading it again. Any other lookup reads
///   as v0's does.
///
/// - the `PrivateDocumentCreateFromShieldedPool` (type 29) `basic_structure`, `Some(0)` rather
///   than `None`: the transition, new at protocol version 15, has its entries, value balance,
///   proof and anchor checked.
///
/// v10 remains unchanged for `PROTOCOL_VERSION_14` chain replay.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            identity_create_state_transition: DriveAbciStateTransitionValidationVersion {
                basic_structure: Some(1), // the identity id is checked before the asset lock proof is verified
                advanced_structure: Some(2), // v1 without the identity id check, which basic structure now makes
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .identity_create_state_transition
            },
            identity_update_state_transition: DriveAbciStateTransitionValidationVersion {
                advanced_structure: Some(1), // added key proof of possession failures are refused unpaid
                state: 2, // an identity keeps at most one enabled Hecate signer key
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .identity_update_state_transition
            },
            masternode_vote_state_transition: DriveAbciStateTransitionValidationVersion {
                transform_into_action: 1, // a vote on a yes/no poll looks up the masternode's previous answer
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .masternode_vote_state_transition
            },
            contract_update_state_transition: DriveAbciStateTransitionValidationVersion {
                transform_into_action: 1, // check_tx holds the update to the compiled size cap
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .contract_update_state_transition
            },
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                advanced_structure: 2, // a tradeable token must be transferable, checked in check_tx too
                document_reference_validation: 1, // a consumed commitment keeps what its lookup read
                document_create_transition_structure_validation: 2, // a decision proposal states its fund, a rule within the limits and an end date one to four weeks away
                document_create_transition_state_validation: 3, // a non-contested create cannot take the values of a live contest its index values do not decide
                data_triggers: DriveAbciValidationDataTriggerAndBindingVersions {
                    bindings: 3, // no DashPay bindings (DashPay v3 declares their checks); the token market contract's orders are written by the marketplace transitions only
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
            private_document_create_from_shielded_pool_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0), // a private document create (new in v15) has a basic structure
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .private_document_create_from_shielded_pool_state_transition
                },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
