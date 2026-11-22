use versioned_feature_core::{FeatureVersion, FeatureVersionBounds, OptionalFeatureVersion};

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

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
    /// How a new document's id is derived. 0 hashes the contract, owner,
    /// document type and entropy; 1 also hashes the identity contract nonce
    /// of the create transition, so an id can be produced at most once.
    pub generate_document_id: FeatureVersion,
    /// The distinct index keys of a typed array property's elements, which an index naming
    /// the array keys the document by (protocol version 14). `None` before: no index can
    /// name a typed array there.
    pub get_raw_array_elements_for_document_type: OptionalFeatureVersion,
    /// Reads a serialized document's id and revision from the fields its serialization format
    /// writes ahead of its properties, without decoding them (`Document::id_and_revision_from_bytes`),
    /// as the protocol version 15 migration of retained document histories does. `None` before:
    /// nothing reads them that way.
    pub id_and_revision_from_bytes: OptionalFeatureVersion,
}
