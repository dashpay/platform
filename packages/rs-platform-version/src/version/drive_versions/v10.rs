use crate::version::drive_versions::drive_document_method_versions::v5::DRIVE_DOCUMENT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{DriveMethodVersions, DriveVersion};

/// Drive version 10, introduced in protocol v15: `DRIVE_DOCUMENT_METHOD_VERSIONS_V5`, where a
/// create deletes the document it consumes from what its `refersTo` lookup read. Everything
/// else is v9's.
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V5,
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
