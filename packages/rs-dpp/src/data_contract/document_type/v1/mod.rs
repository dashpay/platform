use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;

use crate::data_contract::document_type::accessors::DocumentTypeV1Setters;
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use crate::data_contract::document_type::token_costs::accessors::TokenCostSettersV0;
use crate::data_contract::document_type::token_costs::TokenCosts;
use crate::data_contract::document_type::v0::DocumentTypeV0;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::validator::StatelessJsonSchemaLazyValidator;
use crate::document::transfer::Transferable;
use crate::identity::SecurityLevel;
use crate::nft::TradeMode;
use crate::tokens::token_amount_on_contract_token::DocumentActionTokenCost;
use platform_value::{Identifier, Value};

mod accessors;
#[cfg(feature = "random-document-types")]
pub mod random_document_type;

#[derive(Debug, PartialEq, Clone)]
pub struct DocumentTypeV1 {
    pub(in crate::data_contract) name: String,
    pub(in crate::data_contract) schema: Value,
    pub(in crate::data_contract) indices: BTreeMap<String, Index>,
    pub(in crate::data_contract) index_structure: IndexLevel,
    /// Flattened properties flatten all objects for quick lookups for indexes
    /// Document field should not contain sub objects.
    pub(in crate::data_contract) flattened_properties: IndexMap<String, DocumentProperty>,
    /// Document field can contain sub objects.
    pub(in crate::data_contract) properties: IndexMap<String, DocumentProperty>,
    pub(in crate::data_contract) identifier_paths: BTreeSet<String>,
    pub(in crate::data_contract) binary_paths: BTreeSet<String>,
    /// The required fields on the document type
    pub(in crate::data_contract) required_fields: BTreeSet<String>,
    /// The transient fields on the document type
    pub(in crate::data_contract) transient_fields: BTreeSet<String>,
    /// Should documents keep history?
    pub(in crate::data_contract) documents_keep_history: bool,
    /// Are documents mutable?
    pub(in crate::data_contract) documents_mutable: bool,
    /// Can documents of this type be deleted?
    pub(in crate::data_contract) documents_can_be_deleted: bool,
    /// Can documents be transferred without a trade?
    pub(in crate::data_contract) documents_transferable: Transferable,
    /// How are these documents traded?
    pub(in crate::data_contract) trade_mode: TradeMode,
    /// Is document creation restricted?
    pub(in crate::data_contract) creation_restriction_mode: CreationRestrictionMode,
    /// The data contract id
    pub(in crate::data_contract) data_contract_id: Identifier,
    /// Encryption key storage requirements
    pub(in crate::data_contract) requires_identity_encryption_bounded_key:
        Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub(in crate::data_contract) requires_identity_decryption_bounded_key:
        Option<StorageKeyRequirements>,
    pub(in crate::data_contract) security_level_requirement: SecurityLevel,
    #[cfg(feature = "validation")]
    pub(in crate::data_contract) json_schema_validator: StatelessJsonSchemaLazyValidator,
    /// The token costs associated with state transitions on this document type
    pub(in crate::data_contract) token_costs: TokenCosts,
}

impl DocumentTypeBasicMethods for DocumentTypeV1 {}

impl DocumentTypeV0Methods for DocumentTypeV1 {}

impl DocumentTypeV1Setters for DocumentTypeV1 {
    fn set_document_creation_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_creation_token_cost(cost)
    }

    fn set_document_replacement_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_replacement_token_cost(cost)
    }

    fn set_document_deletion_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_deletion_token_cost(cost)
    }

    fn set_document_transfer_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_transfer_token_cost(cost)
    }

    fn set_document_price_update_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_price_update_token_cost(cost)
    }

    fn set_document_purchase_token_cost(&mut self, cost: Option<DocumentActionTokenCost>) {
        self.token_costs.set_document_purchase_token_cost(cost)
    }
}

impl From<DocumentTypeV0> for DocumentTypeV1 {
    fn from(value: DocumentTypeV0) -> Self {
        DocumentTypeV1 {
            name: value.name,
            schema: value.schema,
            indices: value.indices,
            index_structure: value.index_structure,
            flattened_properties: value.flattened_properties,
            properties: value.properties,
            identifier_paths: value.identifier_paths,
            binary_paths: value.binary_paths,
            required_fields: value.required_fields,
            transient_fields: value.transient_fields,
            documents_keep_history: value.documents_keep_history,
            documents_mutable: value.documents_mutable,
            documents_can_be_deleted: value.documents_can_be_deleted,
            documents_transferable: value.documents_transferable,
            trade_mode: value.trade_mode,
            creation_restriction_mode: value.creation_restriction_mode,
            data_contract_id: value.data_contract_id,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            security_level_requirement: value.security_level_requirement,
            #[cfg(feature = "validation")]
            json_schema_validator: value.json_schema_validator,
            token_costs: TokenCosts::V0(Default::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::token_costs::accessors::TokenCostGettersV0;
    use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use crate::tokens::token_amount_on_contract_token::{
        DocumentActionTokenCost, DocumentActionTokenEffect,
    };
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;

    /// Build a `DocumentTypeV0` via `DocumentType::try_from_schema` against an older
    /// PlatformVersion so the `try_from_schema` dispatcher returns V0.
    fn build_v0(schema: Value, transferable: bool, mutable: bool) -> DocumentTypeV0 {
        // Use PlatformVersion::first() which routes to DocumentTypeV0::try_from_schema
        let platform_version = PlatformVersion::first();
        let config =
            DataContractConfig::default_for_version(platform_version).expect("default config");

        // Augment schema with flags controlling test
        let schema = match schema {
            Value::Map(mut map) => {
                map.push((
                    Value::Text("documentsMutable".to_string()),
                    Value::Bool(mutable),
                ));
                if transferable {
                    map.push((Value::Text("transferable".to_string()), Value::U64(1)));
                }
                Value::Map(map)
            }
            other => other,
        };

        let dt = crate::data_contract::document_type::DocumentType::try_from_schema(
            Identifier::new([33; 32]),
            0,
            config.version(),
            "legacy",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut vec![],
            platform_version,
        )
        .expect("v0 doc type should build");

        match dt {
            crate::data_contract::document_type::DocumentType::V0(v0) => v0,
            crate::data_contract::document_type::DocumentType::V1(_)
            | crate::data_contract::document_type::DocumentType::V2(_) => {
                panic!("expected V0 from first() version routing")
            }
        }
    }

    #[test]
    fn from_v0_preserves_all_fields_and_zeroes_token_costs() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 10_u32},
            },
            "additionalProperties": false,
        });
        let v0 = build_v0(schema, true, true);

        // Capture pre-conversion values
        let name = v0.name.clone();
        let data_contract_id = v0.data_contract_id;
        let is_mutable = v0.documents_mutable;
        let transferable = v0.documents_transferable;

        let v1: DocumentTypeV1 = v0.into();

        // Core fields preserved
        assert_eq!(v1.name(), &name);
        assert_eq!(v1.data_contract_id(), data_contract_id);
        assert_eq!(v1.documents_mutable(), is_mutable);
        assert_eq!(v1.documents_transferable(), transferable);

        // Token costs default to all None
        assert!(v1.token_costs.document_creation_token_cost().is_none());
        assert!(v1.token_costs.document_replacement_token_cost().is_none());
        assert!(v1.token_costs.document_deletion_token_cost().is_none());
        assert!(v1.token_costs.document_transfer_token_cost().is_none());
        assert!(v1.token_costs.document_price_update_token_cost().is_none());
        assert!(v1.token_costs.document_purchase_token_cost().is_none());
    }

    #[test]
    fn from_v0_preserves_properties_and_indices() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "first": {"type": "string", "position": 0, "maxLength": 60_u32},
                "second": {"type": "string", "position": 1, "maxLength": 60_u32},
            },
            "indices": [
                {"name": "by_first", "properties": [{"first": "asc"}]},
            ],
            "additionalProperties": false,
        });
        let v0 = build_v0(schema, false, false);
        let original_properties = v0.properties.clone();
        let original_indices = v0.indices.clone();

        let v1: DocumentTypeV1 = v0.into();

        assert_eq!(v1.properties().len(), original_properties.len());
        assert!(v1.properties().contains_key("first"));
        assert!(v1.properties().contains_key("second"));

        assert_eq!(v1.indexes().len(), original_indices.len());
        assert!(v1.indexes().contains_key("by_first"));
    }

    // ----------------------------------------------------------------
    // DocumentTypeV1Setters — cover each setter by calling through the
    // trait on the resulting DocumentTypeV1 value.
    // ----------------------------------------------------------------
    #[test]
    fn setters_update_token_costs_in_place() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 10_u32},
            },
            "additionalProperties": false,
        });
        let mut v1: DocumentTypeV1 = build_v0(schema, false, false).into();

        let cost = DocumentActionTokenCost {
            contract_id: None,
            token_contract_position: 0,
            token_amount: 7,
            effect: DocumentActionTokenEffect::TransferTokenToContractOwner,
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
        };

        v1.set_document_creation_token_cost(Some(cost));
        v1.set_document_replacement_token_cost(Some(cost));
        v1.set_document_deletion_token_cost(Some(cost));
        v1.set_document_transfer_token_cost(Some(cost));
        v1.set_document_price_update_token_cost(Some(cost));
        v1.set_document_purchase_token_cost(Some(cost));

        assert!(v1.token_costs.document_creation_token_cost().is_some());
        assert!(v1.token_costs.document_replacement_token_cost().is_some());
        assert!(v1.token_costs.document_deletion_token_cost().is_some());
        assert!(v1.token_costs.document_transfer_token_cost().is_some());
        assert!(v1.token_costs.document_price_update_token_cost().is_some());
        assert!(v1.token_costs.document_purchase_token_cost().is_some());

        // Clearing a cost should result in None
        v1.set_document_creation_token_cost(None);
        assert!(v1.token_costs.document_creation_token_cost().is_none());
    }

    #[test]
    fn traits_are_implemented_for_document_type_v1() {
        // Sanity-check that DocumentTypeV1 implements DocumentTypeBasicMethods
        // and DocumentTypeV0Methods via the trait system.
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 10_u32},
            },
            "additionalProperties": false,
        });
        let v0 = build_v0(schema, false, true);
        let v1: DocumentTypeV1 = v0.into();

        // Methods from DocumentTypeBasicMethods should be accessible
        assert!(v1.requires_revision()); // mutable
                                         // initial_revision method is present
        assert!(v1.initial_revision().is_some());
    }
}
