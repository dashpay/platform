use crate::data_contract::validation::data_contract_validator::DataContractValidator;
use crate::data_contract::{CreatedDataContract, DataContractFactory};
use crate::prelude::*;
use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub use data_contracts::DataContractSource;
pub use data_contracts::SystemDataContract;

fn create_data_contract_factory() -> DataContractFactory {
    let protocol_version_validator =
        ProtocolVersionValidator::new(LATEST_VERSION, LATEST_VERSION, COMPATIBILITY_MAP.clone());

    let data_contract_validator = DataContractValidator::new(Arc::new(protocol_version_validator));

    DataContractFactory::new(1, Arc::new(data_contract_validator))
}

fn create_data_contract(
    factory: &DataContractFactory,
    system_contract: SystemDataContract,
) -> Result<DataContract, ProtocolError> {
    let DataContractSource {
        id_bytes,
        owner_id_bytes,
        definitions,
        document_schemas,
    } = system_contract
        .source()
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;

    let id = Identifier::from(id_bytes);
    let owner_id = Identifier::from(owner_id_bytes);

    let mut data_contract = factory.create(
        owner_id,
        document_schemas.into(),
        None,
        definitions.map(|def| def.into()),
    )?;

    data_contract.data_contract.id = id;

    Ok(data_contract.data_contract)
}

pub fn load_system_data_contract(
    system_contract: SystemDataContract,
) -> Result<DataContract, ProtocolError> {
    let factory = create_data_contract_factory();

    create_data_contract(&factory, system_contract)
}

pub fn load_system_data_contracts(
    system_contracts: BTreeSet<SystemDataContract>,
) -> Result<BTreeMap<SystemDataContract, DataContract>, ProtocolError> {
    let factory = create_data_contract_factory();

    system_contracts
        .into_iter()
        .map(|system_contract| {
            let data_contract = create_data_contract(&factory, system_contract)?;

            Ok((system_contract, data_contract))
        })
        .collect()
}
