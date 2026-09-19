use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DataContractValidationVersions, DocumentTypeValidationVersions,
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
///
/// `validate_config_update` 2 admits the contract moderation declaration of config V2: a
/// moderation list may be turned on by an update and its moderators changed, but a list
/// that is on can never be turned off.
pub const DPP_VALIDATION_VERSIONS_V5: DPPValidationVersions = DPPValidationVersions {
    // Once-per-identity token distributions: version 1 distribution rules and claims of
    // distribution type 2 exist from this protocol version on.
    data_contract: DataContractValidationVersions {
        validate_config_update: 2,
        validate_once_per_identity_distribution: Some(0),
        ..DPP_VALIDATION_VERSIONS_V4.data_contract
    },
    document_type: DocumentTypeValidationVersions {
        validate_update: 1,
        ..DPP_VALIDATION_VERSIONS_V4.document_type
    },
    ..DPP_VALIDATION_VERSIONS_V4
};
