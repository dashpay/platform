use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DocumentTypeValidationVersions,
};

use super::v5::DPP_VALIDATION_VERSIONS_V5;

/// Protocol v15 validation versions.
///
/// v2 document-type update validation knows the `canBeErased` keyword: it
/// keeps erasability immutable in both directions and lets a keep-history
/// type withdraw deletion only while it is not erasable.
pub const DPP_VALIDATION_VERSIONS_V6: DPPValidationVersions = DPPValidationVersions {
    document_type: DocumentTypeValidationVersions {
        validate_update: 2,
        ..DPP_VALIDATION_VERSIONS_V5.document_type
    },
    ..DPP_VALIDATION_VERSIONS_V5
};
