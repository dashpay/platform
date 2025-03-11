use crate::data_contract::config;
use crate::data_contract::config::v0::{DataContractConfigGettersV0, DataContractConfigSettersV0};
use crate::data_contract::config::{
    DataContractConfig, DEFAULT_CONTRACT_CAN_BE_DELETED, DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED,
    DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY, DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
    DEFAULT_CONTRACT_KEEPS_HISTORY, DEFAULT_CONTRACT_MUTABILITY, DEFAULT_SIZED_INTEGER_TYPES,
};
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct DataContractConfigV1 {
    /// Can the contract ever be deleted. If the contract is deleted, so should be all
    /// documents associated with it. TODO: There should also be a way to "stop" the contract -
    /// contract and documents are kept in the system, but no new documents can be added to it
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
}

/// Trait representing getters for `DataContractConfigV1`
pub trait DataContractConfigGettersV1: DataContractConfigGettersV0 {
    /// Use sized integer Rust types for `integer` property type based on validation rules
    fn sized_integer_types(&self) -> bool;
}

/// Trait representing setters for `DataContractConfigV1`
pub trait DataContractConfigSettersV1: DataContractConfigSettersV0 {
    /// Enable/disable sized integer Rust types for `integer` property type
    fn set_sized_integer_types_enabled(&mut self, enable: bool);
}

impl Default for DataContractConfigV1 {
    fn default() -> Self {
        DataContractConfigV1 {
            can_be_deleted: DEFAULT_CONTRACT_CAN_BE_DELETED,
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
            documents_can_be_deleted_contract_default: DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            sized_integer_types: true,
        }
    }
}

impl DataContractConfigV1 {
    pub fn from_value(value: Value) -> Result<Self, ProtocolError> {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    pub fn default_with_version() -> DataContractConfig {
        Self::default().into()
    }
}

impl DataContractConfigV1 {
    /// Retrieve contract configuration properties.
    ///
    /// This method takes a BTreeMap representing a contract and retrieves
    /// the configuration properties based on the values found in the map.
    ///
    /// The process of retrieving contract configuration properties is versioned,
    /// and the version is determined by the platform version parameter.
    /// If the version is not supported, an error is returned.
    ///
    /// # Parameters
    ///
    /// * `contract`: BTreeMap representing the contract.
    /// * `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// * `Result<ContractConfig, ProtocolError>`: On success, a ContractConfig.
    ///   On failure, a ProtocolError.
    #[inline(always)]
    pub(super) fn get_contract_configuration_properties_v1(
        contract: &BTreeMap<String, Value>,
    ) -> Result<DataContractConfigV1, ProtocolError> {
        let keeps_history = contract
            .get_optional_bool(config::property::KEEPS_HISTORY)?
            .unwrap_or(DEFAULT_CONTRACT_KEEPS_HISTORY);
        let can_be_deleted = contract
            .get_optional_bool(config::property::CAN_BE_DELETED)?
            .unwrap_or(DEFAULT_CONTRACT_CAN_BE_DELETED);

        let readonly = contract
            .get_optional_bool(config::property::READONLY)?
            .unwrap_or(!DEFAULT_CONTRACT_MUTABILITY);

        let documents_keep_history_contract_default = contract
            .get_optional_bool(config::property::DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT)?
            .unwrap_or(DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY);

        let documents_mutable_contract_default = contract
            .get_optional_bool(config::property::DOCUMENTS_MUTABLE_CONTRACT_DEFAULT)?
            .unwrap_or(DEFAULT_CONTRACT_DOCUMENT_MUTABILITY);

        let documents_can_be_deleted_contract_default = contract
            .get_optional_bool(config::property::DOCUMENTS_CAN_BE_DELETED_CONTRACT_DEFAULT)?
            .unwrap_or(DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED);

        let requires_identity_encryption_bounded_key = contract
            .get_optional_integer::<u8>(config::property::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY)?
            .map(|int| int.try_into())
            .transpose()?;

        let requires_identity_decryption_bounded_key = contract
            .get_optional_integer::<u8>(config::property::REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY)?
            .map(|int| int.try_into())
            .transpose()?;

        let sized_integer_types = contract
            .get_optional_bool(config::property::SIZED_INTEGER_TYPES)?
            .unwrap_or(DEFAULT_SIZED_INTEGER_TYPES);

        Ok(DataContractConfigV1 {
            can_be_deleted,
            readonly,
            keeps_history,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
            documents_can_be_deleted_contract_default,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
            sized_integer_types,
        })
    }
}
