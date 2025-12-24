use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentMethodVersions {
    pub query: DriveDocumentQueryMethodVersions,
    pub delete: DriveDocumentDeleteMethodVersions,
    pub insert: DriveDocumentInsertMethodVersions,
    pub insert_contested: DriveDocumentInsertContestedMethodVersions,
    pub update: DriveDocumentUpdateMethodVersions,
    pub estimation_costs: DriveDocumentEstimationCostsMethodVersions,
    pub index_uniqueness: DriveDocumentIndexUniquenessMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentQueryMethodVersions {
    pub query_documents: FeatureVersion,
    pub query_contested_documents: FeatureVersion,
    pub query_contested_documents_vote_state: FeatureVersion,
    pub query_documents_with_flags: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentEstimationCostsMethodVersions {
    pub add_estimation_costs_for_add_document_to_primary_storage: FeatureVersion,
    pub add_estimation_costs_for_add_contested_document_to_primary_storage: FeatureVersion,
    pub stateless_delete_of_non_tree_for_costs: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentInsertMethodVersions {
    pub add_document: FeatureVersion,
    pub add_document_for_contract: FeatureVersion,
    pub add_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_document_for_contract_operations: FeatureVersion,
    pub add_document_to_primary_storage: FeatureVersion,
    pub add_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub add_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub add_reference_for_index_level_for_contract_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentInsertContestedMethodVersions {
    pub add_contested_document: FeatureVersion,
    pub add_contested_document_for_contract: FeatureVersion,
    pub add_contested_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_contested_document_for_contract_operations: FeatureVersion,
    pub add_contested_document_to_primary_storage: FeatureVersion,
    pub add_contested_indices_for_contract_operations: FeatureVersion,
    pub add_contested_reference_and_vote_subtree_to_document_operations: FeatureVersion,
    pub add_contested_vote_subtree_for_non_identities_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentUpdateMethodVersions {
    pub add_update_multiple_documents_operations: FeatureVersion,
    pub update_document_for_contract: FeatureVersion,
    pub update_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub update_document_for_contract_id: FeatureVersion,
    pub update_document_for_contract_operations: FeatureVersion,
    pub update_document_with_serialization_for_contract: FeatureVersion,
    pub update_serialized_document_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentDeleteMethodVersions {
    pub add_estimation_costs_for_remove_document_to_primary_storage: FeatureVersion,
    pub delete_document_for_contract: FeatureVersion,
    pub delete_document_for_contract_id: FeatureVersion,
    pub delete_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub remove_document_from_primary_storage: FeatureVersion,
    pub remove_reference_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub delete_document_for_contract_id_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentIndexUniquenessMethodVersions {
    pub validate_document_create_transition_action_uniqueness: FeatureVersion,
    pub validate_document_replace_transition_action_uniqueness: FeatureVersion,
    pub validate_document_transfer_transition_action_uniqueness: FeatureVersion,
    pub validate_document_purchase_transition_action_uniqueness: FeatureVersion,
    pub validate_document_update_price_transition_action_uniqueness: FeatureVersion,
}
