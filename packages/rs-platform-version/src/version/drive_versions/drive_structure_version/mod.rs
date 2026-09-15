use versioned_feature_core::{FeatureVersion, FeatureVersionBounds};

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DriveStructureVersion {
    pub document_indexes: FeatureVersionBounds,
    pub identity_indexes: FeatureVersionBounds,
    pub pools: FeatureVersionBounds,
    /// How a keep-history document type stores its retained revisions.
    ///
    /// * `0`: every revision lives in the document's own subtree under the
    ///   primary key tree, keyed by block time; the current revision is the
    ///   `[0]` reference inside that subtree.
    /// * `1`: the primary key tree holds only a pointer to the current
    ///   revision, and the revisions live in the type's history tree, one
    ///   provable count tree per document keyed by block time and revision.
    ///
    /// Drive selects paths, references and queries from this value; the
    /// activation hook migrates stored histories when it changes.
    pub keep_history_storage: FeatureVersion,
}
