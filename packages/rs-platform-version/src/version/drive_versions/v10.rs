use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::drive_group_method_versions::v2::DRIVE_GROUP_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::v3::DRIVE_IDENTITY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{
    DriveBatchOperationsMethodVersion, DriveFeesMethodVersions, DriveMethodVersions, DriveVersion,
};

/// Drive version 10.
/// Introduced in protocol v15, carrying the fee-history rules for storage
/// refunds:
///
/// * **Fee history required for refunds** — `fees.calculate_fee` 0 -> 1.
///   The v1 generation consults the block's fee history on every fee
///   version number and returns an internal error for an owner-attributed
///   (sectioned) storage removal without that history, where v0 priced
///   fee version number 1 against an empty map. Every shipped schedule
///   shares number 1 and the same storage rates, so the credits are
///   unchanged; only a missing history is now an error instead of a
///   silent fallback to the first-generation rates.
/// * **Recorded-owner refund credits** — `DRIVE_IDENTITY_METHOD_VERSIONS_V3`
///   turns on `update.credit_storage_refunds_to_owners`, the primitive
///   block lifecycle paths use to credit each recorded owner of a refund
///   and report the amount whose owner has no balance element.
/// * **Price before commit** — `batch_operations.apply_drive_operations`
///   1 -> 2, `DRIVE_GROUP_METHOD_VERSIONS_V2` (`insert.add_group_action`
///   0 -> 1), `DRIVE_CONTRACT_METHOD_VERSIONS_V5` (`update_contract` 2 -> 3,
///   `apply_contract_with_serialization` 0 -> 1, the five moderation
///   writers that can free flagged bytes 0 -> 1) and
///   `DRIVE_DOCUMENT_METHOD_VERSIONS_V5` (the ten fee-returning document
///   wrappers 0 -> 1). Every fee-returning entry point that owns its
///   transaction when the caller passes none now prices the batch before
///   committing, so the missing-history error above never leaves a write
///   persisted without its fee result. With a caller transaction nothing
///   changes.
///
/// Everything else matches `DRIVE_VERSION_V9`.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5, // changed in v10: the ten fee-returning document wrappers price before committing an owned transaction
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5, // changed in v10: update_contract v3, apply_contract_with_serialization v1 and the moderation writers price before committing an owned transaction
        fees: DriveFeesMethodVersions { calculate_fee: 1 }, // changed in v10: fee history required and consulted for every storage refund
        identity: DRIVE_IDENTITY_METHOD_VERSIONS_V3, // changed in v10: credit_storage_refunds_to_owners primitive for lifecycle refund settlement
        batch_operations: DriveBatchOperationsMethodVersion {
            apply_drive_operations: 2, // changed in v10: the batch is priced before an owned transaction commits
            ..DRIVE_VERSION_V9.methods.batch_operations
        },
        group: DRIVE_GROUP_METHOD_VERSIONS_V2, // changed in v10: add_group_action prices before committing an owned transaction
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
