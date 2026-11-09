use crate::version::drive_versions::drive_identity_method_versions::v2::DRIVE_IDENTITY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_identity_method_versions::{
    DriveIdentityKeysFetchMethodVersions, DriveIdentityKeysMethodVersions,
    DriveIdentityMethodVersions,
};

/// V3 is the identity-method table protocol version 15 builds on (`DRIVE_VERSION_V10` also
/// raises two cost estimators on top of it). It differs from V2 only in
/// `keys.fetch.fetch_identity_hecate_signer_key_ids` `None -> Some(0)`: the identity update
/// that adds a Hecate signer key reads the identity's signer index, which holds its enabled
/// signer keys, to refuse a second one. No key has the `HECATE_SIGNER` purpose before protocol
/// version 15, so no earlier identity has the index.
pub const DRIVE_IDENTITY_METHOD_VERSIONS_V3: DriveIdentityMethodVersions =
    DriveIdentityMethodVersions {
        keys: DriveIdentityKeysMethodVersions {
            fetch: DriveIdentityKeysFetchMethodVersions {
                fetch_identity_hecate_signer_key_ids: Some(0), // changed in v3
                ..DRIVE_IDENTITY_METHOD_VERSIONS_V2.keys.fetch
            },
            ..DRIVE_IDENTITY_METHOD_VERSIONS_V2.keys
        },
        ..DRIVE_IDENTITY_METHOD_VERSIONS_V2
    };
