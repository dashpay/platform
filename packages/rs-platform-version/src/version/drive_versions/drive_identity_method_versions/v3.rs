use crate::version::drive_versions::drive_identity_method_versions::v2::DRIVE_IDENTITY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::{
    DriveIdentityMethodVersions, DriveIdentityUpdateMethodVersions,
};

/// V3 is protocol version 15's identity-method table. It differs from V2 in
/// one slot:
///
/// * `update.credit_storage_refunds_to_owners` `None -> Some(0)`: the
///   primitive that credits each recorded owner of a storage refund and
///   reports the amount whose owner has no balance element, so a block
///   lifecycle path (or, later, the state transition refund path) can route
///   that amount to the current epoch's processing pool instead of halting.
///   No key or permission is consulted on any route: a frozen but existing
///   owner is credited, an owner without a balance is settled into the pool.
///   Nothing before v15 settles refunds outside a state transition, so V1 and
///   V2 keep the slot `None`.
pub const DRIVE_IDENTITY_METHOD_VERSIONS_V3: DriveIdentityMethodVersions =
    DriveIdentityMethodVersions {
        update: DriveIdentityUpdateMethodVersions {
            credit_storage_refunds_to_owners: Some(0), // new in v15: credits recorded refund owners, reports the unrouted amount for the processing pool
            ..DRIVE_IDENTITY_METHOD_VERSIONS_V2.update
        },
        ..DRIVE_IDENTITY_METHOD_VERSIONS_V2
    };
