#[cfg(feature = "validation")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "validation")]
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;

mod create_document_types_from_document_schemas;
mod try_from_schema;

#[inline]
fn consensus_or_protocol_data_contract_error(
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
fn consensus_or_protocol_value_error(platform_value_error: platform_value::Error) -> ProtocolError {
    #[cfg(feature = "validation")]
    {
        ProtocolError::ConsensusError(
            ConsensusError::BasicError(BasicError::ValueError(platform_value_error.into())).into(),
        )
    }
    #[cfg(not(feature = "validation"))]
    {
        ProtocolError::ValueError(platform_value_error.into())
    }
}
