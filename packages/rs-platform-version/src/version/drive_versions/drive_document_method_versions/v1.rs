use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentIndexUniquenessMethodVersions, DriveDocumentInsertContestedMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V1: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        query: DriveDocumentQueryMethodVersions {
            query_documents: 0,
            query_contested_documents: 0,
            query_contested_documents_vote_state: 0,
            query_documents_with_flags: 0,
        },
        delete: DriveDocumentDeleteMethodVersions {
            add_estimation_costs_for_remove_document_to_primary_storage: 0,
            delete_document_for_contract: 0,
            delete_document_for_contract_id: 0,
            delete_document_for_contract_apply_and_add_to_operations: 0,
            remove_document_from_primary_storage: 0,
            remove_reference_for_index_level_for_contract_operations: 0,
            remove_indices_for_index_level_for_contract_operations: 0,
            remove_indices_for_top_index_level_for_contract_operations: 0,
            delete_document_for_contract_id_with_named_type_operations: 0,
            delete_document_for_contract_with_named_type_operations: 0,
            delete_document_for_contract_operations: 0,
        },
        insert: DriveDocumentInsertMethodVersions {
            add_document: 0,
            add_document_for_contract: 0,
            add_document_for_contract_apply_and_add_to_operations: 0,
            add_document_for_contract_operations: 0,
            add_document_to_primary_storage: 0,
            add_indices_for_index_level_for_contract_operations: 0,
            add_indices_for_top_index_level_for_contract_operations: 0,
            add_reference_for_index_level_for_contract_operations: 0,
        },
        insert_contested: DriveDocumentInsertContestedMethodVersions {
            add_contested_document: 0,
            add_contested_document_for_contract: 0,
            add_contested_document_for_contract_apply_and_add_to_operations: 0,
            add_contested_document_for_contract_operations: 0,
            add_contested_document_to_primary_storage: 0,
            add_contested_indices_for_contract_operations: 0,
            add_contested_reference_and_vote_subtree_to_document_operations: 0,
            add_contested_vote_subtree_for_non_identities_operations: 0,
        },
        update: DriveDocumentUpdateMethodVersions {
            add_update_multiple_documents_operations: 0,
            update_document_for_contract: 0,
            update_document_for_contract_apply_and_add_to_operations: 0,
            update_document_for_contract_id: 0,
            update_document_for_contract_operations: 0,
            update_document_with_serialization_for_contract: 0,
            update_serialized_document_for_contract: 0,
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            add_estimation_costs_for_add_document_to_primary_storage: 0,
            add_estimation_costs_for_add_contested_document_to_primary_storage: 0,
            stateless_delete_of_non_tree_for_costs: 0,
        },
        index_uniqueness: DriveDocumentIndexUniquenessMethodVersions {
            validate_document_uniqueness: 0,
            validate_document_create_transition_action_uniqueness: 0,
            validate_document_replace_transition_action_uniqueness: 0,
            validate_document_transfer_transition_action_uniqueness: 0,
            validate_document_purchase_transition_action_uniqueness: 0,
            validate_document_update_price_transition_action_uniqueness: 0,
            validate_uniqueness_of_data: 0,
        },
    };