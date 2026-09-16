use crate::version::dpp_versions::dpp_validation_versions::{
    DPPValidationVersions, DocumentTypeValidationVersions,
};

use super::v5::DPP_VALIDATION_VERSIONS_V5;

/// Protocol v17 validation versions.
///
/// Turns on `validate_contested_index_parameters` (`Some(0)`): a contested
/// index registered or updated at this version may only name parameters the
/// native contest machinery can honour. Every contested index property must be
/// a top-level, required, user-defined property, and every `fieldMatches`
/// entry must name a string property of that index. Each rule closes a way
/// for a declaration the parser accepts to break at contest time: a nested or
/// system property yields an empty vote poll key, an optional property reaches
/// the contested tree walker as a null, a match on a non-index property lets
/// two documents with equal index values take different insert paths so the
/// later award collides in the unique index during block execution, and a
/// match on a non-string property never matches so the contest can never
/// start.
///
/// Stored contracts are parsed without full validation and never run the
/// check; only contract create and update transitions validated at v17 or
/// later can be rejected by it. Everything else matches
/// `DPP_VALIDATION_VERSIONS_V5`.
pub const DPP_VALIDATION_VERSIONS_V6: DPPValidationVersions = DPPValidationVersions {
    document_type: DocumentTypeValidationVersions {
        validate_contested_index_parameters: Some(0),
        ..DPP_VALIDATION_VERSIONS_V5.document_type
    },
    ..DPP_VALIDATION_VERSIONS_V5
};
