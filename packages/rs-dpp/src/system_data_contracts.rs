use crate::data_contract::DataContractFactory;
use crate::prelude::*;
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::accessors::v0::DataContractV0Setters;
use crate::data_contract::config::v1::DataContractConfigSettersV1;
use crate::data_contract::config::DataContractConfig;
pub use data_contracts::*;
use platform_version::version::PlatformVersion;

pub trait ConfigurationForSystemContract {
    fn configuration_in_platform_version(
        &self,
        version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError>;
}

impl ConfigurationForSystemContract for SystemDataContract {
    fn configuration_in_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match self {
            SystemDataContract::Withdrawals => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::MasternodeRewards => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            // Reserved slot with no implementation. Any caller that reaches here
            // has a bug (they should have short-circuited on `source()` returning
            // `ContractReserved`). Return a harmless default config rather than
            // panicking so this failure mode stays non-fatal.
            SystemDataContract::FeatureFlags => {
                DataContractConfig::default_for_version(platform_version)
            }
            SystemDataContract::DPNS => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::Dashpay => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::WalletUtils => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::TokenHistory => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::KeywordSearch => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::DocumentHistory => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::AppConnect => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
        }
    }
}

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

    let mut data_contract = factory.create(
        owner_id,
        0,
        document_schemas.into(),
        Some(system_contract.configuration_in_platform_version(platform_version)?),
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
            .try_into_platform_versioned(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v2_ser: DataContractInSerializationFormat = contract_2
            .clone()
            .try_into_platform_versioned(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_ser, v2_ser);

        let v1_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1757); // this will still use a config v0 without sized_integer_types

        let v1_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1758); // this will use a config v1 in serialization with sized_integer_types
    }
}

#[cfg(all(test, feature = "app-connect-contract", feature = "validation"))]
mod app_connect_tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use crate::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use platform_value::Value;

    fn response(contract: &DataContract) -> Document {
        let mut document = contract
            .document_type_for_name("loginKeyResponse")
            .expect("response type")
            .random_document(Some(42), PlatformVersion::latest())
            .expect("response document");
        document.set_properties(BTreeMap::from([
            (
                "appEphemeralPubKeyHash".into(),
                Value::Bytes(vec![0x11; 20]),
            ),
            ("walletEphemeralPubKey".into(), Value::Bytes(vec![0x22; 33])),
            ("encryptedPayload".into(), Value::Bytes(vec![0x33; 60])),
        ]));
        document
    }

    #[test]
    fn should_validate_app_connect_response_lengths() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::AppConnect, platform_version)
            .expect("system contract");
        for (property, length, expected) in [
            ("appEphemeralPubKeyHash", 19, false),
            ("appEphemeralPubKeyHash", 20, true),
            ("appEphemeralPubKeyHash", 21, false),
            ("walletEphemeralPubKey", 32, false),
            ("walletEphemeralPubKey", 33, true),
            ("walletEphemeralPubKey", 34, false),
            ("encryptedPayload", 59, false),
            ("encryptedPayload", 60, true),
            ("encryptedPayload", 572, true),
            ("encryptedPayload", 573, false),
        ] {
            let mut document = response(&contract);
            document.set(property, Value::Bytes(vec![0x44; length]));
            let result = contract
                .validate_document("loginKeyResponse", &document, platform_version)
                .expect("validation executes");
            assert_eq!(
                result.is_valid(),
                expected,
                "{property}: {length} bytes: {result:?}"
            );
        }
    }

    #[test]
    fn should_require_only_the_three_app_connect_response_properties() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::AppConnect, platform_version)
            .expect("system contract");
        assert_eq!(contract.document_types().len(), 1);
        for property in [
            "appEphemeralPubKeyHash",
            "walletEphemeralPubKey",
            "encryptedPayload",
        ] {
            let mut document = response(&contract);
            document.properties_mut().remove(property);
            assert!(
                !contract
                    .validate_document("loginKeyResponse", &document, platform_version)
                    .expect("validation executes")
                    .is_valid(),
                "{property} is required"
            );
        }
        let mut document = response(&contract);
        document.set("contractId", Value::Bytes(vec![0x55; 32]));
        assert!(
            !contract
                .validate_document("loginKeyResponse", &document, platform_version)
                .expect("validation executes")
                .is_valid(),
            "the former contractId field is not admitted"
        );
    }
}
