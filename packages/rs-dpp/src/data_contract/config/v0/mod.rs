use crate::data_contract::config;
use crate::data_contract::config::{
    DataContractConfig, DEFAULT_CONTRACT_CAN_BE_DELETED, DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
    DEFAULT_CONTRACT_DOCUMENT_MUTABILITY, DEFAULT_CONTRACT_KEEPS_HISTORY,
    DEFAULT_CONTRACT_MUTABILITY,
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
pub struct DataContractConfigV0 {
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
    /// Are documents in the contract mutable. This specifies whether the document can be
    /// changed or deleted. This is a default for all documents in the contract, but can be
    /// overridden by the document itself
    pub documents_mutable_contract_default: bool,
    /// Encryption key storage requirements
    pub requires_identity_encryption_bounded_key: Option<StorageKeyRequirements>,
    /// Decryption key storage requirements
    pub requires_identity_decryption_bounded_key: Option<StorageKeyRequirements>,
}

/// Trait representing getters for `DataContractConfigV0`
pub trait DataContractConfigGettersV0 {
    /// Returns whether the contract can be deleted.
    fn can_be_deleted(&self) -> bool;

    /// Returns whether the contract is read-only.
    fn readonly(&self) -> bool;

    /// Returns whether the contract keeps history.
    fn keeps_history(&self) -> bool;

    /// Returns whether documents in the contract keep history by default.
    fn documents_keep_history_contract_default(&self) -> bool;

    /// Returns whether documents in the contract are mutable by default.
    fn documents_mutable_contract_default(&self) -> bool;

    /// Encryption key storage requirements
    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements>;

    /// Decryption key storage requirements
    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements>;
}

/// Trait representing setters for `DataContractConfigV0`
pub trait DataContractConfigSettersV0 {
    /// Sets whether the contract can be deleted.
    fn set_can_be_deleted(&mut self, value: bool);

    /// Sets whether the contract is read-only.
    fn set_readonly(&mut self, value: bool);

    /// Sets whether the contract keeps history.
    fn set_keeps_history(&mut self, value: bool);

    /// Sets whether documents in the contract keep history by default.
    fn set_documents_keep_history_contract_default(&mut self, value: bool);

    /// Sets whether documents in the contract are mutable by default.
    fn set_documents_mutable_contract_default(&mut self, value: bool);

    /// Sets Encryption key storage requirements.
    fn set_requires_identity_encryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    );

    /// Sets Decryption key storage requirements.
    fn set_requires_identity_decryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    );
}

impl std::default::Default for DataContractConfigV0 {
    fn default() -> Self {
        DataContractConfigV0 {
            can_be_deleted: DEFAULT_CONTRACT_CAN_BE_DELETED,
            readonly: !DEFAULT_CONTRACT_MUTABILITY,
            keeps_history: DEFAULT_CONTRACT_KEEPS_HISTORY,
            documents_keep_history_contract_default: DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
            documents_mutable_contract_default: DEFAULT_CONTRACT_DOCUMENT_MUTABILITY,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
        }
    }
}

impl DataContractConfigV0 {
    pub fn from_value(value: Value) -> Result<Self, ProtocolError> {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    pub fn default_with_version() -> DataContractConfig {
        Self::default().into()
    }
}

impl DataContractConfigV0 {
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
    pub(super) fn get_contract_configuration_properties_v0(
        contract: &BTreeMap<String, Value>,
    ) -> Result<DataContractConfigV0, ProtocolError> {
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

        let requires_identity_encryption_bounded_key = contract
            .get_optional_integer::<u8>(config::property::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY)?
            .map(|int| int.try_into())
            .transpose()?;

        let requires_identity_decryption_bounded_key = contract
            .get_optional_integer::<u8>(config::property::REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY)?
            .map(|int| int.try_into())
            .transpose()?;

        Ok(DataContractConfigV0 {
            can_be_deleted,
            readonly,
            keeps_history,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
            requires_identity_encryption_bounded_key,
            requires_identity_decryption_bounded_key,
        })
    }
}
