use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV0MutGetters, DocumentTypeV0Setters, DocumentTypeV1Getters,
};
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::accessors::TokenCostGettersV0;
use crate::data_contract::document_type::v1::DocumentTypeV1;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::TokenContractPosition;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

impl DocumentTypeV0MutGetters for DocumentTypeV1 {
    fn schema_mut(&mut self) -> &mut Value {
        &mut self.schema
    }
}

impl DocumentTypeV0Getters for DocumentTypeV1 {
    fn name(&self) -> &String {
        &self.name
    }

    fn schema(&self) -> &Value {
        &self.schema
    }

    fn schema_owned(self) -> Value {
        self.schema
    }

    fn indexes(&self) -> &BTreeMap<String, Index> {
        &self.indices
    }

    fn find_contested_index(&self) -> Option<&Index> {
        self.indices
            .iter()
            .find(|(_, index)| index.contested_index.is_some())
            .map(|(_, contested_index)| contested_index)
    }

    fn index_structure(&self) -> &IndexLevel {
        &self.index_structure
    }

    fn flattened_properties(&self) -> &IndexMap<String, DocumentProperty> {
        &self.flattened_properties
    }

    fn properties(&self) -> &IndexMap<String, DocumentProperty> {
        &self.properties
    }

    fn identifier_paths(&self) -> &BTreeSet<String> {
        &self.identifier_paths
    }

    fn binary_paths(&self) -> &BTreeSet<String> {
        &self.binary_paths
    }

    fn required_fields(&self) -> &BTreeSet<String> {
        &self.required_fields
    }
    fn transient_fields(&self) -> &BTreeSet<String> {
        &self.transient_fields
    }

    fn documents_keep_history(&self) -> bool {
        self.documents_keep_history
    }

    fn documents_mutable(&self) -> bool {
        self.documents_mutable
    }

    fn documents_can_be_deleted(&self) -> bool {
        self.documents_can_be_deleted
    }

    fn documents_transferable(&self) -> Transferable {
        self.documents_transferable
    }

    fn trade_mode(&self) -> TradeMode {
        self.trade_mode
    }

    fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        self.creation_restriction_mode
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        self.requires_identity_encryption_bounded_key
    }

    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        self.requires_identity_decryption_bounded_key
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        self.security_level_requirement
    }

    #[cfg(feature = "validation")]
    fn json_schema_validator_ref(&self) -> &StatelessJsonSchemaLazyValidator {
        &self.json_schema_validator
    }
}

impl DocumentTypeV0Setters for DocumentTypeV1 {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        self.data_contract_id = data_contract_id;
    }
}

impl DocumentTypeV1Getters for DocumentTypeV1 {
    fn document_creation_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_creation_token_cost()
    }

    fn document_replacement_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_replacement_token_cost()
    }

    fn document_deletion_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_deletion_token_cost()
    }

    fn document_transfer_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_transfer_token_cost()
    }

    fn document_update_price_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_price_update_token_cost()
    }

    fn document_purchase_token_cost(&self) -> Option<DocumentActionTokenCost> {
        self.token_costs.document_purchase_token_cost()
    }

    fn all_document_token_costs(&self) -> Vec<&DocumentActionTokenCost> {
        let mut result = Vec::new();

        if let Some(cost) = self.token_costs.document_creation_token_cost_ref() {
            result.push(cost);
        }
        if let Some(cost) = self.token_costs.document_replacement_token_cost_ref() {
            result.push(cost);
        }
        if let Some(cost) = self.token_costs.document_deletion_token_cost_ref() {
            result.push(cost);
        }
        if let Some(cost) = self.token_costs.document_transfer_token_cost_ref() {
            result.push(cost);
        }
        if let Some(cost) = self.token_costs.document_price_update_token_cost_ref() {
            result.push(cost);
        }
        if let Some(cost) = self.token_costs.document_purchase_token_cost_ref() {
            result.push(cost);
        }

        result
    }

    fn all_external_token_costs_contract_tokens(
        &self,
    ) -> BTreeMap<Identifier, BTreeSet<TokenContractPosition>> {
        let mut map = BTreeMap::new();

        for cost in self.all_document_token_costs() {
            if let Some(contract_id) = cost.contract_id {
                map.entry(contract_id)
                    .or_insert_with(BTreeSet::new)
                    .insert(cost.token_contract_position);
            }
        }

        map
    }
}
