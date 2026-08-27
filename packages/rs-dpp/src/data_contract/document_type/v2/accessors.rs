use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV0MutGetters, DocumentTypeV0Setters, DocumentTypeV1Getters,
    DocumentTypeV2Getters, DocumentTypeV2Setters,
};
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;

use platform_value::{Identifier, Value};

use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::accessors::TokenCostGettersV0;
use crate::data_contract::document_type::v2::DocumentTypeV2;
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

impl DocumentTypeV0MutGetters for DocumentTypeV2 {
    fn schema_mut(&mut self) -> &mut Value {
        &mut self.schema
    }
}

impl DocumentTypeV0Getters for DocumentTypeV2 {
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

    fn documents_keep_transfer_history(&self) -> bool {
        self.documents_keep_transfer_history
    }

    fn documents_keep_purchase_history(&self) -> bool {
        self.documents_keep_purchase_history
    }

    fn documents_keep_pricing_history(&self) -> bool {
        self.documents_keep_pricing_history
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

impl DocumentTypeV0Setters for DocumentTypeV2 {
    fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        self.data_contract_id = data_contract_id;
    }
}

impl DocumentTypeV1Getters for DocumentTypeV2 {
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

impl DocumentTypeV2Getters for DocumentTypeV2 {
    fn documents_countable(&self) -> bool {
        self.documents_countable || self.range_countable
    }

    fn range_countable(&self) -> bool {
        self.range_countable
    }

    fn documents_summable(&self) -> Option<&str> {
        self.documents_summable.as_deref()
    }

    fn range_summable(&self) -> bool {
        self.range_summable
    }

    fn index_only(&self) -> bool {
        self.index_only
    }
}

impl DocumentTypeV2Setters for DocumentTypeV2 {
    fn set_documents_countable(&mut self, countable: bool) {
        self.documents_countable = countable;
        if !countable {
            // Preserve invariant: range_countable implies documents_countable
            self.range_countable = false;
        }
    }

    fn set_range_countable(&mut self, range_countable: bool) {
        self.range_countable = range_countable;
        if range_countable {
            self.documents_countable = true;
        }
    }

    fn set_documents_summable(&mut self, property: Option<String>) {
        let cleared = property.is_none();
        self.documents_summable = property;
        if cleared {
            // Preserve invariant: range_summable requires
            // documents_summable.is_some()
            self.range_summable = false;
        }
    }

    fn set_range_summable(&mut self, range_summable: bool) {
        // Normalize unconditionally: `range_summable` requires a property
        // to sum on, so clamp to false when `documents_summable` is unset.
        // This way an existing-true-but-inconsistent state can't survive
        // a setter call — the invariant always holds after this returns.
        self.range_summable = range_summable && self.documents_summable.is_some();
    }

    fn set_index_only(&mut self, index_only: bool) {
        self.index_only = index_only;
    }
}
