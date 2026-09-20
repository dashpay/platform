use crate::consensus::basic::data_contract::UnknownGasFeesPaidByError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::Display;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display, DecodeUntrusted)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
/// Who pays the gas (the storage and processing fee) of a document action that is paid for
/// with a token.
///
/// The same enum is used on both sides of a token payment. On a document type's token cost it
/// is what the contract owner offers; on a transition's token payment info it is what the
/// document owner asks for. [`GasFeesPaidBy::resolve`] combines the two into the effective payer
/// from protocol version 14 on: before it the field was carried but never acted on, and the
/// signer always paid.
///
/// The contract owner can only be asked to pay when the action actually charges a token, so
/// every sponsored transition is backed by a token the contract owner chose to hand out.
pub enum GasFeesPaidBy {
    /// The document owner pays the gas fees.
    ///
    /// On the contract side this is the default and means the contract owner never pays. On the
    /// transition side it opts out of any sponsorship the contract offers.
    #[default]
    DocumentOwner = 0,
    /// The contract owner pays the gas fees.
    ///
    /// On the contract side this offers to pay, and accepts every request. On the transition
    /// side this insists that the contract owner pays: the transition is refused, and nobody is
    /// charged, when the contract owner cannot cover the fee, and it is rejected outright when
    /// the contract offers less than that.
    ContractOwner = 1,
    /// The contract owner pays the gas fees when their balance covers them; otherwise the
    /// document owner does.
    ///
    /// On the contract side this offers to pay but refuses to be the reason a transition fails,
    /// so a transition insisting on `ContractOwner` is rejected. On the transition side it is
    /// the document owner stating their willingness to pay the fee themselves when the contract
    /// owner's balance is insufficient, and it is accepted by every contract.
    PreferContractOwner = 2,
}

impl GasFeesPaidBy {
    /// The effective gas payer of a document action, given what the document type's token
    /// cost offers (`offered_by_contract`) and what the transition's token payment info asks
    /// for (`requested`), or `None` when the request cannot be honoured.
    ///
    /// | offered \ requested   | `DocumentOwner` | `PreferContractOwner` | `ContractOwner` |
    /// |-----------------------|-----------------|-----------------------|-----------------|
    /// | `DocumentOwner`       | document owner  | document owner        | refused         |
    /// | `PreferContractOwner` | document owner  | prefer contract owner | refused         |
    /// | `ContractOwner`       | document owner  | prefer contract owner | contract owner  |
    ///
    /// An action without a token cost offers `DocumentOwner`, and a transition without token
    /// payment info requests it.
    pub fn resolve(offered_by_contract: Self, requested: Self) -> Option<Self> {
        match (offered_by_contract, requested) {
            (_, GasFeesPaidBy::DocumentOwner) => Some(GasFeesPaidBy::DocumentOwner),
            (GasFeesPaidBy::DocumentOwner, GasFeesPaidBy::PreferContractOwner) => {
                Some(GasFeesPaidBy::DocumentOwner)
            }
            (GasFeesPaidBy::DocumentOwner, GasFeesPaidBy::ContractOwner) => None,
            (GasFeesPaidBy::PreferContractOwner, GasFeesPaidBy::PreferContractOwner) => {
                Some(GasFeesPaidBy::PreferContractOwner)
            }
            (GasFeesPaidBy::PreferContractOwner, GasFeesPaidBy::ContractOwner) => None,
            (GasFeesPaidBy::ContractOwner, GasFeesPaidBy::PreferContractOwner) => {
                Some(GasFeesPaidBy::PreferContractOwner)
            }
            (GasFeesPaidBy::ContractOwner, GasFeesPaidBy::ContractOwner) => {
                Some(GasFeesPaidBy::ContractOwner)
            }
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for GasFeesPaidBy {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for GasFeesPaidBy {}

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
mod resolve_tests {
    use super::GasFeesPaidBy::{ContractOwner, DocumentOwner, PreferContractOwner};
    use super::*;

    #[test]
    fn should_let_the_document_owner_opt_out_of_any_offer() {
        for offered in [DocumentOwner, PreferContractOwner, ContractOwner] {
            assert_eq!(
                GasFeesPaidBy::resolve(offered, DocumentOwner),
                Some(DocumentOwner)
            );
        }
    }

    #[test]
    fn should_accept_a_preference_from_every_contract() {
        assert_eq!(
            GasFeesPaidBy::resolve(DocumentOwner, PreferContractOwner),
            Some(DocumentOwner)
        );
        assert_eq!(
            GasFeesPaidBy::resolve(PreferContractOwner, PreferContractOwner),
            Some(PreferContractOwner)
        );
        assert_eq!(
            GasFeesPaidBy::resolve(ContractOwner, PreferContractOwner),
            Some(PreferContractOwner)
        );
    }

    #[test]
    fn should_honour_an_insistence_only_when_the_contract_commits_to_paying() {
        assert_eq!(GasFeesPaidBy::resolve(DocumentOwner, ContractOwner), None);
        assert_eq!(
            GasFeesPaidBy::resolve(PreferContractOwner, ContractOwner),
            None
        );
        assert_eq!(
            GasFeesPaidBy::resolve(ContractOwner, ContractOwner),
            Some(ContractOwner)
        );
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    // `GasFeesPaidBy` is a unit-only enum without `rename_all`, so each variant
    // (de)serializes as its PascalCase Rust name in both JSON and platform_value.

    #[test]
    fn json_round_trip_document_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::DocumentOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("DocumentOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_contract_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::ContractOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("ContractOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_round_trip_prefer_contract_owner() {
        use crate::serialization::JsonConvertible;
        let original = GasFeesPaidBy::PreferContractOwner;
        let json = original.to_json().expect("to_json");
        assert_eq!(json, json!("PreferContractOwner"));
        let recovered = GasFeesPaidBy::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_document_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::DocumentOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("DocumentOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_contract_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::ContractOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("ContractOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_prefer_contract_owner() {
        use crate::serialization::ValueConvertible;
        let original = GasFeesPaidBy::PreferContractOwner;
        let value = original.to_object().expect("to_object");
        assert_eq!(value, platform_value!("PreferContractOwner"));
        let recovered = GasFeesPaidBy::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
