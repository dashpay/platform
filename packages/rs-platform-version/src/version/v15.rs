use crate::version::dpp_versions::dpp_contract_versions::v7::CONTRACT_VERSIONS_V7;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v4::STATE_TRANSITION_SERIALIZATION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_validation_versions::v6::DPP_VALIDATION_VERSIONS_V6;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v4::DRIVE_ABCI_QUERY_VERSIONS_V4;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v11::DRIVE_ABCI_VALIDATION_VERSIONS_V11;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_limits::v5::SYSTEM_LIMITS_V5;
use crate::version::v14::PLATFORM_V14;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// Protocol v15 moves keep-history documents to per-type history trees and
/// gives them a delete and erase lifecycle.
///
/// 1. **Per-type history trees.** The primary-key entry stores the current
///    document while retained revisions live in a separate provable count tree
///    keyed by block time and revision. The first v15 block migrates existing
///    v14 histories and index references before v15 state transitions execute.
///    History queries and proofs select the matching layout through Drive's
///    version tables.
/// 2. **Keep-history delete.** A keep-history document type may now allow
///    deletion. A delete removes the document's current pointer and index
///    references and records a lifecycle entry; the retained revisions stay
///    readable through the history query, which reports the deleted state and
///    its time. Creating a document under a deleted id is refused until its
///    history is gone.
/// 3. **Erase.** A document type that keeps history and can be deleted may
///    declare `canBeErased` (meta-schema v4, parser generation 4; the keyword
///    is immutable across contract updates). An erase transition removes the
///    retained revisions of a deleted document a chunk of
///    `max_document_revisions_erased_per_transition` at a time: the owner
///    authorizes the first chunk, any identity may submit a continuation, and
///    the last chunk removes the lifecycle entry, after which the id reads as
///    absent. The history query reports the erasing state, its start time and
///    the remaining revision count.
/// 4. **Wire.** The erase kind is appended to the batch's document transition
///    enum, so a batch of either shipped wire format may carry one. The batch
///    basic-structure gate admits the kind only where this protocol version
///    publishes bounds for it, which keeps new software agreeing with software
///    that cannot decode the kind while an earlier version is still active.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: document v5 and contract v5 write the per-type history tree and the delete/erase lifecycle; verify v3 authenticates history pages and lifecycle counts; state transition methods v5 convert the lifecycle delete and the erase
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: the protocol-change hook v2 migrates retained histories on the first v15 block
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V11, // changed: delete consults the lifecycle; erase structure and state validation
        query: DRIVE_ABCI_QUERY_VERSIONS_V4, // changed: the history handler reports the deleted and erasing lifecycle states with their times
        ..PLATFORM_V14.drive_abci
    },
    dpp: DPPVersion {
        validation: DPP_VALIDATION_VERSIONS_V6, // changed: document type update validation v2 keeps `canBeErased` immutable
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V4, // changed: the erase kind joins the wire
        contract_versions: CONTRACT_VERSIONS_V7, // changed: v4 document meta-schema and parser generation 4 admit deletable keep-history types and `canBeErased`
        ..PLATFORM_V14.dpp
    },
    system_limits: SYSTEM_LIMITS_V5, // changed: erase chunk of 100 revisions per transition
    ..PLATFORM_V14
};
