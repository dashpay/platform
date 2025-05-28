use crate::data_contract::{DataContract, DataContractFactory};
use crate::errors::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::accessors::v0::DataContractV0Setters;
pub use data_contracts::*;
use platform_value::Identifier;
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
        0,
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
    let factory = DataContractFactory::new(platform_version.protocol_version)?;

    create_data_contract(&factory, system_contract, platform_version)
}

pub fn load_system_data_contracts(
    system_contracts: BTreeSet<SystemDataContract>,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<SystemDataContract, DataContract>, ProtocolError> {
    let factory = DataContractFactory::new(platform_version.protocol_version)?;

    system_contracts
        .into_iter()
        .map(|system_contract| {
            let data_contract = create_data_contract(&factory, system_contract, platform_version)?;

            Ok((system_contract, data_contract))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use platform_version::TryIntoPlatformVersioned;
    #[test]
    fn test_load_system_data_contract_v8_vs_v9() {
        let contract_1 = load_system_data_contract(
            SystemDataContract::TokenHistory,
            PlatformVersion::get(8).unwrap(),
        )
        .expect("data_contract");
        let contract_2 = load_system_data_contract(
            SystemDataContract::TokenHistory,
            PlatformVersion::get(9).unwrap(),
        )
        .expect("data_contract");
        assert_ne!(contract_1, contract_2);
    }

    #[test]
    fn serialize_withdrawal_contract_v1_vs_v9() {
        let contract_1 = load_system_data_contract(
            SystemDataContract::Withdrawals,
            PlatformVersion::get(1).unwrap(),
        )
        .expect("data_contract");
        let contract_2 = load_system_data_contract(
            SystemDataContract::Withdrawals,
            PlatformVersion::get(9).unwrap(),
        )
        .expect("data_contract");

        assert_ne!(contract_1, contract_2);
        let v1_ser: DataContractInSerializationFormat = contract_1
            .clone()
            .try_into_platform_versioned(&PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v2_ser: DataContractInSerializationFormat = contract_2
            .clone()
            .try_into_platform_versioned(&PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_ser, v2_ser);

        let v1_bytes = contract_1
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_1
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_1
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1757); // this will still use a config v0 without sized_integer_types

        let v1_bytes = contract_2
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_2
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_2
            .serialize_to_bytes_with_platform_version(&PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1758); // this will use a config v1 in serialization with sized_integer_types
    }
}
