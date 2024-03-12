use crate::drive::identity::contract_info::keys::IdentityDataContractKeyApplyInfo::ContractBased;
use crate::drive::Drive;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::{KeyID, Purpose};
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod add_potential_contract_info_for_contract_bounded_key;

pub enum IdentityDataContractKeyApplyInfo {
    /// The root_id is either a contract id or an owner id
    /// It is a contract id for in the case of contract bound keys or contract
    /// document bound keys
    ContractBased {
        contract_id: Identifier,
        document_type_keys: BTreeMap<String, Vec<(KeyID, Purpose)>>,
        contract_keys: Vec<(KeyID, Purpose)>,
    },
    // ContractFamilyBased {
    //     contracts_owner_id: Identifier,
    //     family_keys: Vec<KeyID>,
    // },
}

impl IdentityDataContractKeyApplyInfo {
    fn root_id(&self) -> [u8; 32] {
        match self {
            ContractBased { contract_id, .. } => contract_id.to_buffer(),
            // ContractFamilyBased {
            //     contracts_owner_id, ..
            // } => contracts_owner_id.to_buffer(),
        }
    }
    fn keys(
        self,
    ) -> (
        BTreeMap<String, Vec<(KeyID, Purpose)>>,
        Vec<(KeyID, Purpose)>,
    ) {
        match self {
            ContractBased {
                document_type_keys,
                contract_keys,
                ..
            } => (document_type_keys, contract_keys),
            // ContractFamilyBased { family_keys, .. } => (BTreeMap::new(), family_keys),
        }
    }
    fn new_from_single_key(
        key_id: KeyID,
        purpose: Purpose,
        contract_bounds: &ContractBounds,
        drive: &Drive,
        epoch: &Epoch,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let contract_id = contract_bounds.identifier().to_buffer();
        // we are getting with fetch info to add the cost to the drive operations
        let maybe_contract_fetch_info = drive.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            Some(epoch),
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
        let Some(contract_fetch_info) = maybe_contract_fetch_info else {
            return Err(Error::Identity(IdentityError::IdentityKeyBoundsError(
                "Contract for key bounds not found",
            )));
        };
        let contract = &contract_fetch_info.contract;
        match contract_bounds {
            ContractBounds::SingleContract { .. } => Ok(ContractBased {
                contract_id: contract.id(),
                document_type_keys: Default::default(),
                contract_keys: vec![(key_id, purpose)],
            }),
            ContractBounds::SingleContractDocumentType {
                document_type_name: document_type,
                ..
            } => {
                let document_type = contract.document_type_for_name(document_type)?;
                Ok(ContractBased {
                    contract_id: contract.id(),
                    document_type_keys: BTreeMap::from([(
                        document_type.name().clone(),
                        vec![(key_id, purpose)],
                    )]),
                    contract_keys: vec![],
                })
            } // ContractBounds::MultipleContractsOfSameOwner { .. } => Ok(ContractFamilyBased {
              //     contracts_owner_id: contract.owner_id(),
              //     family_keys: vec![key_id],
              // }),
        }
    }
}
