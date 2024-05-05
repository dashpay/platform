use crate::consensus::basic::data_contract::UnknownTradeModeError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TradeMode {
    None = 0,
    DirectPurchase = 1,
    // PublicOffer = 2,
    // PrivateOffer = 3,
}

impl TradeMode {
    pub fn seller_sets_price(&self) -> bool {
        match self {
            TradeMode::None => false,
            TradeMode::DirectPurchase => true,
            // TradeMode::PublicOffer => true,  //min price
            // TradeMode::PrivateOffer => true, //min price
        }
    }
}

impl Display for TradeMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TradeMode::None => write!(f, "No Trading"),
            TradeMode::DirectPurchase => write!(f, "Direct Purchase"),
            // TradeMode::PublicOffer => write!(f, "Public Offer"),
            // TradeMode::PrivateOffer => write!(f, "Private Offer"),
        }
    }
}

impl TryFrom<u8> for TradeMode {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::DirectPurchase),
            // 2 => Ok(Self::PublicOffer),
            // 3 => Ok(Self::PrivateOffer),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownTradeModeError(
                    UnknownTradeModeError::new(vec![0, 1], value),
                ))
                .into(),
            )),
        }
    }
}
