use crate::balances::credits::TokenAmount;
use crate::consensus::basic::data_contract::UnknownDocumentActionTokenEffectError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::ProtocolError;
use platform_value::Identifier;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum DocumentActionTokenEffect {
    /// When the document action occurs the token is transferred to the balance of the contract owner
    #[default]
    TransferTokenToContractOwner,
    /// When the document action occurs the token is burned
    /// This option is not available for external tokens.
    BurnToken,
}

impl From<DocumentActionTokenEffect> for u8 {
    fn from(value: DocumentActionTokenEffect) -> Self {
        match value {
            DocumentActionTokenEffect::TransferTokenToContractOwner => 0,
            DocumentActionTokenEffect::BurnToken => 1,
        }
    }
}

impl TryFrom<u8> for DocumentActionTokenEffect {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DocumentActionTokenEffect::TransferTokenToContractOwner),
            1 => Ok(DocumentActionTokenEffect::BurnToken),
            other => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownDocumentActionTokenEffectError(
                    UnknownDocumentActionTokenEffectError::new(vec![0, 1], other as u64),
                ))
                .into(),
            )),
        }
    }
}

impl TryFrom<u64> for DocumentActionTokenEffect {
    type Error = ProtocolError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        u8::try_from(value)
            .map_err(|_| {
                ProtocolError::ConsensusError(
                    ConsensusError::BasicError(BasicError::UnknownDocumentActionTokenEffectError(
                        UnknownDocumentActionTokenEffectError::new(vec![0, 1], value),
                    ))
                    .into(),
                )
            })?
            .try_into()
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct DocumentActionTokenCost {
    /// If this is not set, it means that we are using our own contract id
    pub contract_id: Option<Identifier>,
    /// Token contract position
    pub token_contract_position: TokenContractPosition,
    /// The amount
    pub token_amount: TokenAmount,
    /// The amount
    pub effect: DocumentActionTokenEffect,
    /// Who is paying for gas fees for this action
    pub gas_fees_paid_by: GasFeesPaidBy,
}
