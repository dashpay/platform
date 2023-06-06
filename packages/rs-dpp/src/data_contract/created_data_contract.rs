use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::property_names::SYSTEM_VERSION;
use crate::data_contract::v0::created_data_contract::CreatedDataContractV0;
use crate::version::FeatureVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;

#[derive(Clone, Debug, From)]
pub enum CreatedDataContract {
    V0(CreatedDataContractV0),
}

impl CreatedDataContract {
    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(mut raw_object: Value) -> Result<Self, ProtocolError> {
        let data_contract_system_version =
            match raw_object.remove_optional_integer::<FeatureVersion>(SYSTEM_VERSION) {
                Ok(Some(data_contract_system_version)) => data_contract_system_version,
                Ok(None) => {
                    return Err(ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::VersionError(
                            "no system version found on data contract object".into(),
                        ))
                        .into(),
                    ));
                }
                Err(e) => {
                    return Err(ProtocolError::ConsensusError(
                        ConsensusError::BasicError(BasicError::VersionError(
                            format!("version error: {}", e.to_string()).into(),
                        ))
                        .into(),
                    ));
                }
            };
        match data_contract_system_version {
            0 => Ok(CreatedDataContractV0::from_raw_object(raw_object).into()),
            _ => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::VersionError(
                    "system version found on data contract object".into(),
                ))
                .into(),
            )),
        }
    }
}
