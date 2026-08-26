use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;

pub(crate) mod apply_required_since;
mod create_document_types_from_document_schemas;
mod should_use_creator_id;
mod system_properties;
mod try_from_schema;

#[inline]
pub(crate) fn consensus_or_protocol_data_contract_error(
    data_contract_error: DataContractError,
) -> ProtocolError {
    #[cfg(feature = "validation")]
    {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ContractError(data_contract_error)).into(),
        )
    }
    #[cfg(not(feature = "validation"))]
    {
        ProtocolError::DataContractError(data_contract_error)
    }
}

#[inline]
pub(crate) fn consensus_or_protocol_required_fields_error(
    error: DataContractInvalidRequiredFieldsUpdateError,
) -> ProtocolError {
    #[cfg(feature = "validation")]
    {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::DataContractInvalidRequiredFieldsUpdateError(
                error,
            ))
            .into(),
        )
    }
    #[cfg(not(feature = "validation"))]
    {
        ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            error.to_string(),
        ))
    }
}

#[inline]
pub(crate) fn consensus_or_protocol_value_error(
    platform_value_error: platform_value::Error,
) -> ProtocolError {
    #[cfg(feature = "validation")]
    {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ValueError(platform_value_error.into())).into(),
        )
    }
    #[cfg(not(feature = "validation"))]
    {
        ProtocolError::ValueError(platform_value_error)
    }
}
