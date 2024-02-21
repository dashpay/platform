use crate::data_contract::DataContractFactory;
use crate::prelude::*;
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::accessors::v0::DataContractV0Setters;
pub use data_contracts::*;
use platform_version::version::PlatformVersion;

fn create_data_contract(
    factory: &DataContractFactory,
    system_contract: SystemDataContract,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let DataContractSource {
        id_bytes,
        owner_id_bytes,
        version,
        definitions,
        document_schemas,
    } = system_contract
        .source(platform_version)
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;

    let id = Identifier::from(id_bytes);
    let owner_id = Identifier::from(owner_id_bytes);

    let mut data_contract = factory.create_with_value_config(
        owner_id,
        document_schemas.into(),
        None,
        definitions.map(|def| def.into()),
    )?;

    data_contract.data_contract_mut().set_id(id);
    data_contract.data_contract_mut().set_version(version);

    Ok(data_contract.data_contract_owned())
}

pub fn load_system_data_contract(
    system_contract: SystemDataContract,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let factory = DataContractFactory::new(platform_version.protocol_version, None)?;

    create_data_contract(&factory, system_contract, platform_version)
}

pub fn load_system_data_contracts(
    system_contracts: BTreeSet<SystemDataContract>,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<SystemDataContract, DataContract>, ProtocolError> {
    let factory = DataContractFactory::new(platform_version.protocol_version, None)?;

    system_contracts
        .into_iter()
        .map(|system_contract| {
            let data_contract = create_data_contract(&factory, system_contract, platform_version)?;

            Ok((system_contract, data_contract))
        })
        .collect()
}
