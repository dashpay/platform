use crate::data_contract::DataContractFactory;
use crate::prelude::*;
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

pub use data_contracts::*;

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

    data_contract.set_data_contract_id(id);

    Ok(data_contract.data_contract_owned())
}

pub fn load_system_data_contract(
    system_contract: SystemDataContract,
    protocol_version: u32,
) -> Result<DataContract, ProtocolError> {
    let factory = DataContractFactory::new(protocol_version, None)?;

    create_data_contract(&factory, system_contract)
}

pub fn load_system_data_contracts(
    system_contracts: BTreeSet<SystemDataContract>,
    protocol_version: u32,
) -> Result<BTreeMap<SystemDataContract, DataContract>, ProtocolError> {
    let factory = DataContractFactory::new(protocol_version, None)?;

    system_contracts
        .into_iter()
        .map(|system_contract| {
            let data_contract = create_data_contract(&factory, system_contract)?;

            Ok((system_contract, data_contract))
        })
        .collect()
}
