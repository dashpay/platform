use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use crate::ProtocolError;
use bincode_derive::{Decode, Encode};
use derive_more::{Display, From};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::Error;
use platform_value::{Identifier, Value};
#[cfg(any(
    feature = "state-transition-serde-conversion",
    all(
        feature = "document-serde-conversion",
        feature = "data-contract-serde-conversion"
    ),
))]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod methods;
pub mod v0;

#[derive(
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    Display,
    From,
)]
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
pub enum TokenPaymentInfo {
    #[display("V0({})", "_0")]
    V0(TokenPaymentInfoV0),
}

impl TokenPaymentInfoMethodsV0 for TokenPaymentInfo {}

impl TokenPaymentInfoAccessorsV0 for TokenPaymentInfo {
    // Getters
    fn payment_token_contract_id(&self) -> Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id(),
        }
    }

    fn payment_token_contract_id_ref(&self) -> &Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id_ref(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenPaymentInfo::V0(v0) => v0.token_contract_position(),
        }
    }

    fn minimum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.minimum_token_cost(),
        }
    }

    fn maximum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.maximum_token_cost(),
        }
    }

    fn gas_fees_paid_by(&self) -> GasFeesPaidBy {
        match self {
            TokenPaymentInfo::V0(v0) => v0.gas_fees_paid_by(),
        }
    }

    // Setters
    fn set_payment_token_contract_id(&mut self, id: Option<Identifier>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_payment_token_contract_id(id),
        }
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_token_contract_position(position),
        }
    }

    fn set_minimum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_minimum_token_cost(cost),
        }
    }

    fn set_maximum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_maximum_token_cost(cost),
        }
    }

    fn set_gas_fees_paid_by(&mut self, payer: GasFeesPaidBy) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_gas_fees_paid_by(payer),
        }
    }
}

impl TryFrom<BTreeMap<String, Value>> for TokenPaymentInfo {
    type Error = ProtocolError;

    fn try_from(map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        let format_version = map.get_str("$format_version")?;
        match format_version {
            "0" => {
                let token_payment_info: TokenPaymentInfoV0 = map.try_into()?;

                Ok(token_payment_info.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPaymentInfo::from_value".to_string(),
                known_versions: vec![0],
                received: version
                    .parse()
                    .map_err(|_| ProtocolError::Generic("Conversion error".to_string()))?,
            }),
        }
    }
}

#[cfg(feature = "state-transition-value-conversion")]
impl TryFrom<TokenPaymentInfo> for Value {
    type Error = Error;
    fn try_from(value: TokenPaymentInfo) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}
