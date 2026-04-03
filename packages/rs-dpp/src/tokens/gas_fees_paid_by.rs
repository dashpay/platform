use crate::consensus::basic::data_contract::UnknownGasFeesPaidByError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::Display;
#[cfg(any(
    feature = "serde-conversion",
    all(feature = "serde-conversion", feature = "serde-conversion"),
))]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    any(
        feature = "serde-conversion",
        all(feature = "serde-conversion", feature = "serde-conversion"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_gas_fees_paid_by_to_u8_document_owner() {
        assert_eq!(u8::from(GasFeesPaidBy::DocumentOwner), 0);
    }

    #[test]
    fn from_gas_fees_paid_by_to_u8_contract_owner() {
        assert_eq!(u8::from(GasFeesPaidBy::ContractOwner), 1);
    }

    #[test]
    fn from_gas_fees_paid_by_to_u8_prefer_contract_owner() {
        assert_eq!(u8::from(GasFeesPaidBy::PreferContractOwner), 2);
    }

    #[test]
    fn try_from_u8_valid_values() {
        assert_eq!(
            GasFeesPaidBy::try_from(0u8).unwrap(),
            GasFeesPaidBy::DocumentOwner
        );
        assert_eq!(
            GasFeesPaidBy::try_from(1u8).unwrap(),
            GasFeesPaidBy::ContractOwner
        );
        assert_eq!(
            GasFeesPaidBy::try_from(2u8).unwrap(),
            GasFeesPaidBy::PreferContractOwner
        );
    }

    #[test]
    fn try_from_u8_invalid_value_returns_error() {
        let result = GasFeesPaidBy::try_from(3u8);
        assert!(result.is_err());
        let result = GasFeesPaidBy::try_from(255u8);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_u64_valid_values() {
        assert_eq!(
            GasFeesPaidBy::try_from(0u64).unwrap(),
            GasFeesPaidBy::DocumentOwner
        );
        assert_eq!(
            GasFeesPaidBy::try_from(1u64).unwrap(),
            GasFeesPaidBy::ContractOwner
        );
        assert_eq!(
            GasFeesPaidBy::try_from(2u64).unwrap(),
            GasFeesPaidBy::PreferContractOwner
        );
    }

    #[test]
    fn try_from_u64_invalid_small_value_returns_error() {
        let result = GasFeesPaidBy::try_from(3u64);
        assert!(result.is_err());
    }

    #[test]
    fn try_from_u64_large_value_returns_error() {
        let result = GasFeesPaidBy::try_from(256u64);
        assert!(result.is_err());
        let result = GasFeesPaidBy::try_from(u64::MAX);
        assert!(result.is_err());
    }

    #[test]
    fn display_variants() {
        assert_eq!(format!("{}", GasFeesPaidBy::DocumentOwner), "DocumentOwner");
        assert_eq!(format!("{}", GasFeesPaidBy::ContractOwner), "ContractOwner");
        assert_eq!(
            format!("{}", GasFeesPaidBy::PreferContractOwner),
            "PreferContractOwner"
        );
    }

    #[test]
    fn default_is_document_owner() {
        assert_eq!(GasFeesPaidBy::default(), GasFeesPaidBy::DocumentOwner);
    }

    #[test]
    fn u8_round_trip_all_variants() {
        for variant in [
            GasFeesPaidBy::DocumentOwner,
            GasFeesPaidBy::ContractOwner,
            GasFeesPaidBy::PreferContractOwner,
        ] {
            let byte_val: u8 = variant.into();
            let recovered = GasFeesPaidBy::try_from(byte_val).unwrap();
            assert_eq!(variant, recovered);
        }
    }
}
