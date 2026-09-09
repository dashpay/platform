use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DocumentTypeValidationVersions,
};

use super::v4::DPP_VALIDATION_VERSIONS_V4;

/// Protocol v14 validation versions.
///
/// v1 document-type update validation compares index definitions by name
/// instead of comparing `IndexLevel` trees whose level identifiers are
/// assigned by an iteration-order counter. In v0 the counter follows the
/// index-name sort order, so whether a rejected index change surfaced as a
/// proper consensus error or as an opaque "Invalid path" / internal error
/// depended on where the changed index's name sorted relative to the
/// document type's other indexes.
pub const DPP_VALIDATION_VERSIONS_V5: DPPValidationVersions = DPPValidationVersions {
    document_type: DocumentTypeValidationVersions {
        validate_update: 1,
        ..DPP_VALIDATION_VERSIONS_V4.document_type
    },
    ..DPP_VALIDATION_VERSIONS_V4
};
