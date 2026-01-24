#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;

mod create_document_types_from_document_schemas;
mod parse_property_reference;
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
pub(crate) fn consensus_or_protocol_data_contract_error_from_wrapped_protocol_error(
    data_contract_error: ProtocolError,
) -> ProtocolError {
    #[cfg(feature = "validation")]
    {
        match data_contract_error {
            ProtocolError::DataContractError(err) => ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::ContractError(err)).into(),
            ),
            err => err,
        }
    }
    #[cfg(not(feature = "validation"))]
    {
        data_contract_error
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
