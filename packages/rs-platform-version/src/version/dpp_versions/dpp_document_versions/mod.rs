use versioned_feature_core::{FeatureVersion, FeatureVersionBounds};

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPDocumentVersions {
    // This is for the overall structure of the document, like DocumentV0
    pub document_structure_version: FeatureVersion,
    pub document_serialization_version: FeatureVersionBounds,
    pub document_cbor_serialization_version: FeatureVersionBounds,
    pub extended_document_structure_version: FeatureVersion,
    pub extended_document_serialization_version: FeatureVersionBounds,
    pub document_method_versions: DocumentMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentMethodVersions {
    pub is_equal_ignoring_timestamps: FeatureVersion,
    pub hash: FeatureVersion,
    pub get_raw_for_contract: FeatureVersion,
    pub get_raw_for_document_type: FeatureVersion,
    pub try_into_asset_unlock_base_transaction_info: FeatureVersion,
}
