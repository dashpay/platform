use crate::data_contract::config;
use crate::data_contract::config::moderation::ContractModerationConfig;
use crate::data_contract::config::v1::{DataContractConfigGettersV1, DataContractConfigV1};
use crate::data_contract::config::{
    DataContractConfig, DEFAULT_CONTRACT_CAN_BE_DELETED, DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED,
    DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY, DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
    DEFAULT_CONTRACT_KEEPS_HISTORY, DEFAULT_CONTRACT_MUTABILITY, DEFAULT_SIZED_INTEGER_TYPES,
};
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Config V2 (protocol version 14): V1 plus the optional contract moderation declaration.
///
/// A config whose `moderation` is `None` is stored as V1 so that unmoderated contracts keep the
/// bytes they had before (`DataContractConfig::config_valid_for_platform_version`).
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, DecodeUntrusted)]
#[serde(rename_all = "camelCase", default)]
pub struct DataContractConfigV2 {
    /// Can the contract ever be deleted. If the contract is deleted, so should be all
    /// documents associated with it.
    pub can_be_deleted: bool,
    /// Is the contract mutable. Means that the document definitions can be changed or new
    /// document definitions can be added to the contract
    pub readonly: bool,
    /// Does the contract keep history when the contract itself changes
    pub keeps_history: bool,
    /// Do documents in the contract keep history. This is a default for all documents in
    /// the contract, but can be overridden by the document itself
    pub documents_keep_history_contract_default: bool,
    /// Are documents in the contract mutable? This specifies whether the documents can be
    /// changed. This is a default for all document types in the contract, but can be
    /// overridden by the document type config.
    pub documents_mutable_contract_default: bool,
    /// Can documents in the contract be deleted? This specifies whether the documents can be
    /// deleted. This is a default for all document types in the contract, but can be
    /// overridden by the document types itself.
    pub documents_can_be_deleted_contract_default: bool,
    /// Encryption key storage requirements
    pub requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
    /// Use sized integer Rust types for `integer` property type based on validation rules
    pub sized_integer_types: bool,
    /// The moderation lists the contract keeps and who edits them, `None` for an unmoderated
    /// contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<ContractModerationConfig>,
}

/// Trait representing getters for `DataContractConfigV2`
pub trait DataContractConfigGettersV2: DataContractConfigGettersV1 {
    /// The moderation declaration, `None` for an unmoderated contract (and for every config
    /// below V2).
    fn moderation(&self) -> Option<&ContractModerationConfig>;
}

impl Default for DataContractConfigV2 {
    fn default() -> Self {
        DataContractConfigV2 {
            can_be_deleted: DEFAULT_CONTRACT_CAN_BE_DELETED,
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
            documents_can_be_deleted_contract_default: DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            sized_integer_types: DEFAULT_SIZED_INTEGER_TYPES,
            moderation: None,
        }
    }
}

impl DataContractConfigV2 {
    pub fn default_with_version() -> DataContractConfig {
        Self::default().into()
    }

    /// Retrieve contract configuration properties from a contract value map: the V1 properties
    /// plus the optional `moderation` map.
    #[inline(always)]
    pub(super) fn get_contract_configuration_properties_v2(
        contract: &BTreeMap<String, Value>,
    ) -> Result<DataContractConfigV2, ProtocolError> {
        let v1 = DataContractConfigV1::get_contract_configuration_properties_v1(contract)?;
        let moderation = contract
            .get(config::property::MODERATION)
            .filter(|value| !value.is_null())
            .map(|value| platform_value::from_value::<ContractModerationConfig>(value.clone()))
            .transpose()?;
        Ok(DataContractConfigV2 {
            moderation,
            ..v1.into()
        })
    }
}

impl From<DataContractConfigV1> for DataContractConfigV2 {
    fn from(value: DataContractConfigV1) -> Self {
        DataContractConfigV2 {
            can_be_deleted: value.can_be_deleted,
            readonly: value.readonly,
            keeps_history: value.keeps_history,
            documents_keep_history_contract_default: value.documents_keep_history_contract_default,
            documents_mutable_contract_default: value.documents_mutable_contract_default,
            documents_can_be_deleted_contract_default: value
                .documents_can_be_deleted_contract_default,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            sized_integer_types: value.sized_integer_types,
            moderation: None,
        }
    }
}

impl From<DataContractConfigV2> for DataContractConfigV1 {
    /// Drops the moderation declaration. Only meant for a V2 whose `moderation` is `None`.
    fn from(value: DataContractConfigV2) -> Self {
        DataContractConfigV1 {
            can_be_deleted: value.can_be_deleted,
            readonly: value.readonly,
            keeps_history: value.keeps_history,
            documents_keep_history_contract_default: value.documents_keep_history_contract_default,
            documents_mutable_contract_default: value.documents_mutable_contract_default,
            documents_can_be_deleted_contract_default: value
                .documents_can_be_deleted_contract_default,
            requires_identity_encryption_bounded_key: value
                .requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key: value
                .requires_identity_decryption_bounded_key,
            sized_integer_types: value.sized_integer_types,
        }
    }
}
