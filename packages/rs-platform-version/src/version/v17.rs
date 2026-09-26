use crate::version::dpp_versions::dpp_validation_versions::v6::DPP_VALIDATION_VERSIONS_V6;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v10::DRIVE_VERSION_V10;
use crate::version::protocol_version::PlatformVersion;
use crate::version::v16::PLATFORM_V16;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_17: ProtocolVersion = 17;

/// The 5.0 protocol version, the one that introduces smart contracts. Provisional number from
/// the DashVM allocation register (15 for 4.3, 16 for 4.4, 17 for 5.0).
///
/// Changes over v16 so far, all three for contested resource vote polls:
///
/// * `DPP_VALIDATION_VERSIONS_V6` turns on `validate_contested_index_parameters`: a contested
///   index registered or updated at this version may only declare parameters the native
///   contest machinery can honour (top-level, required, non-transient user properties; field
///   matches naming string properties of the index that classify the two strings stored
///   under one key alike). Stored contracts are never re-judged.
/// * `DRIVE_VERSION_V10` turns on `award_contested_document_vote_poll`, the native award
///   operation: it re-derives the ended poll's winner from state and inserts the winning
///   document in one Drive call, takes no contender, and rejects any call that names a wrong
///   end date, a poll that has not ended or a poll already finalized. The slot is `None` on
///   every earlier table.
/// * `DRIVE_ABCI_METHOD_VERSIONS_V11` selects `check_for_ended_vote_polls` v1, which finalizes
///   every ended poll through that operation, then records and cleans up on the same block
///   transaction as before.
///
/// Ordinary document actions on a contested document type keep their ordinary rules (data
/// triggers, creation restrictions and, when they arrive, native guards and predicates); the
/// award is a native block event that runs none of them.
pub const PLATFORM_V17: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_17,
    drive: DRIVE_VERSION_V10, // changed: the native contested award operation
    drive_abci: DriveAbciVersion {
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: the finalization event awards through Drive
        ..PLATFORM_V16.drive_abci
    },
    dpp: DPPVersion {
        validation: DPP_VALIDATION_VERSIONS_V6, // changed: contested index parameter validation
        ..PLATFORM_V16.dpp
    },
    ..PLATFORM_V16
};
