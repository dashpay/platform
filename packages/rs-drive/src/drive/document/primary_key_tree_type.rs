use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TreeType;

use crate::error::drive::DriveError;
use crate::error::Error;

/// Extension trait for `DocumentTypeRef` that provides the tree type used
/// for primary key storage in Drive.
pub trait DocumentTypePrimaryKeyTreeType {
    /// Returns the `TreeType` used for the primary key storage tree.
    ///
    /// The primary key tree (key `[0]` under the document type path) stores
    /// document references keyed by document ID. The tree type depends on the
    /// document type's configuration:
    ///
    /// - `range_countable = true` → `ProvableCountTree`
    /// - `documents_countable = true` → `CountTree`
    /// - otherwise → `NormalTree`
    fn primary_key_tree_type(&self, platform_version: &PlatformVersion) -> Result<TreeType, Error>;
}

impl DocumentTypePrimaryKeyTreeType for DocumentTypeRef<'_> {
    fn primary_key_tree_type(&self, platform_version: &PlatformVersion) -> Result<TreeType, Error> {
        match platform_version
            .drive
            .methods
            .document
            .primary_key_tree_type
        {
            0 => {
                if self.range_countable() {
                    Ok(TreeType::ProvableCountTree)
                } else if self.documents_countable() {
                    Ok(TreeType::CountTree)
                } else {
                    Ok(TreeType::NormalTree)
                }
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentTypeRef::primary_key_tree_type".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Setters;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::PlatformVersion;

    fn make_doc_type() -> DocumentType {
        let pv = PlatformVersion::latest();
        let contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            false,
            pv,
        )
        .expect("contract");
        let dt = contract
            .document_type_for_name("person")
            .expect("person type");
        dt.to_owned_document_type()
    }

    #[test]
    fn default_is_normal_tree() {
        let dt = make_doc_type();
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::NormalTree);
    }

    #[test]
    fn countable_is_count_tree() {
        let mut dt = make_doc_type();
        dt.set_documents_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::CountTree);
    }

    #[test]
    fn blast_is_provable_count_tree() {
        let mut dt = make_doc_type();
        dt.set_range_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::ProvableCountTree);
    }

    #[test]
    fn blast_takes_priority_over_countable() {
        let mut dt = make_doc_type();
        dt.set_documents_countable(true);
        dt.set_range_countable(true);
        let pv = PlatformVersion::latest();
        let result = dt.as_ref().primary_key_tree_type(pv).unwrap();
        assert_eq!(result, TreeType::ProvableCountTree);
    }
}
