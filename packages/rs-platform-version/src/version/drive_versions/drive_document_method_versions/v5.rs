use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentInsertMethodVersions,
    DriveDocumentMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// V5 is protocol version 15's document-method table. Relative to
/// [`super::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4`], the six fee-returning wrappers
/// (`add_document_for_contract`, `delete_document_for_contract`,
/// `delete_document_for_contract_id`, `update_document_for_contract`,
/// `update_document_for_contract_id` and `update_document_with_serialization_for_contract`)
/// are bumped to `1`: when the caller passes no transaction they write and price inside one
/// owned transaction and commit it only once `Drive::calculate_fee` succeeded. Pricing an
/// owner-attributed storage removal without the fee history is an error from this version,
/// and generation 0 committed the write before that error surfaced. The operation builders
/// and the `_apply_and_add_to_operations` methods every production caller uses through
/// `apply_drive_operations` are unchanged.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        insert: DriveDocumentInsertMethodVersions {
            add_document_for_contract: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert
        },
        update: DriveDocumentUpdateMethodVersions {
            update_document_for_contract: 1,
            update_document_for_contract_id: 1,
            update_document_with_serialization_for_contract: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.update
        },
        delete: DriveDocumentDeleteMethodVersions {
            delete_document_for_contract: 1,
            delete_document_for_contract_id: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
