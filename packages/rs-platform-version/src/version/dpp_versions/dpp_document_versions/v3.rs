use crate::version::dpp_versions::dpp_document_versions::{
    DPPDocumentVersions, DocumentMethodVersions,
};
use versioned_feature_core::FeatureVersionBounds;

pub const DOCUMENT_VERSIONS_V3: DPPDocumentVersions = DPPDocumentVersions {
    document_structure_version: 0,
    document_serialization_version: FeatureVersionBounds {
        min_version: 0,
        max_version: 2,
        default_current_version: 2,
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
