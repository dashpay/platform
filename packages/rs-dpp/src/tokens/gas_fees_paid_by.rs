use crate::consensus::basic::data_contract::UnknownGasFeesPaidByError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode_derive::{Decode, Encode};
use derive_more::Display;
#[cfg(any(
    feature = "state-transition-serde-conversion",
    all(
        feature = "document-serde-conversion",
        feature = "data-contract-serde-conversion"
    ),
))]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    any(
        feature = "state-transition-serde-conversion",
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
    ),
    derive(Serialize, Deserialize)
)]
pub enum GasFeesPaidBy {
    /// The user pays the gas fees
    #[default]
    DocumentOwner = 0,
    /// The contract owner pays the gas fees
    ContractOwner = 1,
    /// The user is stating his willingness to pay the gas fee if the Contract owner's balance is
    /// insufficient.
    PreferContractOwner = 2,
}

impl From<GasFeesPaidBy> for u8 {
    fn from(value: GasFeesPaidBy) -> Self {
        match value {
            GasFeesPaidBy::DocumentOwner => 0,
            GasFeesPaidBy::ContractOwner => 1,
            GasFeesPaidBy::PreferContractOwner => 2,
        }
    }
}

impl TryFrom<u8> for GasFeesPaidBy {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(GasFeesPaidBy::DocumentOwner),
            1 => Ok(GasFeesPaidBy::ContractOwner),
            2 => Ok(GasFeesPaidBy::PreferContractOwner),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownGasFeesPaidByError(
                    UnknownGasFeesPaidByError::new(vec![0, 1, 2], value as u64),
                ))
                .into(),
            )),
        }
    }
}

impl TryFrom<u64> for GasFeesPaidBy {
    type Error = ProtocolError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value)
            .map_err(|_| {
                ProtocolError::ConsensusError(
                    ConsensusError::BasicError(BasicError::UnknownGasFeesPaidByError(
                        UnknownGasFeesPaidByError::new(vec![0, 1, 2], value),
                    ))
                    .into(),
                )
            })?
            .try_into()
    }
}
