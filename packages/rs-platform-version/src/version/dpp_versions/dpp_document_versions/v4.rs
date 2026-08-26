use crate::version::dpp_versions::dpp_document_versions::{
    DPPDocumentVersions, DocumentMethodVersions,
};
use versioned_feature_core::FeatureVersionBounds;

/// Document serialization moves to format 3: the document is stamped with the
/// data contract version its bytes conform to, right after the format prefix.
/// The stamp lets a property carry `requiredSince` (required from a given
/// contract version) while documents written before that version keep the
/// presence-flagged layout they were serialized with. Formats 0-2 predate the
/// stamp and deserialize with an unstamped (pre-annotation) layout.
pub const DOCUMENT_VERSIONS_V4: DPPDocumentVersions = DPPDocumentVersions {
    document_structure_version: 0,
    document_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 3,
        default_current_version: 3,
    },
    document_cbor_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    extended_document_structure_version: 0,
    extended_document_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 0,
        default_current_version: 0,
    },
    document_method_versions: DocumentMethodVersions {
        is_equal_ignoring_timestamps: 0,
        hash: 0,
        get_raw_for_contract: 0,
        get_raw_for_document_type: 0,
        try_into_asset_unlock_base_transaction_info: 0,
    },
};
