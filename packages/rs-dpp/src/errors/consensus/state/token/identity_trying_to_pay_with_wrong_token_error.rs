use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[platform_serialize(unversioned)]
pub struct IdentityTryingToPayWithWrongTokenError {
    expected_contract_id: Option<Identifier>,
    expected_token_contract_position: TokenContractPosition,
    expected_token_id: Identifier,

    actual_contract_id: Option<Identifier>,
    actual_token_contract_position: TokenContractPosition,
    actual_token_id: Identifier,
}

impl IdentityTryingToPayWithWrongTokenError {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        expected_contract_id: Option<Identifier>,
        expected_token_contract_position: TokenContractPosition,
        expected_token_id: Identifier,
        actual_contract_id: Option<Identifier>,
        actual_token_contract_position: TokenContractPosition,
        actual_token_id: Identifier,
    ) -> Self {
        Self {
            expected_contract_id,
            expected_token_contract_position,
            expected_token_id,
            actual_contract_id,
            actual_token_contract_position,
            actual_token_id,
        }
    }

    pub fn expected_contract_id(&self) -> Option<Identifier> {
        self.expected_contract_id
    }

    pub fn expected_token_contract_position(&self) -> TokenContractPosition {
        self.expected_token_contract_position
    }

    pub fn expected_token_id(&self) -> &Identifier {
        &self.expected_token_id
    }

    pub fn actual_contract_id(&self) -> Option<Identifier> {
        self.actual_contract_id
    }

    pub fn actual_token_contract_position(&self) -> TokenContractPosition {
        self.actual_token_contract_position
    }

    pub fn actual_token_id(&self) -> &Identifier {
        &self.actual_token_id
    }
}

impl std::fmt::Display for IdentityTryingToPayWithWrongTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expected_contract_display = match &self.expected_contract_id {
            Some(id) => format!("{}", id),
            None => "Not Expected".to_string(),
        };

        let actual_contract_display = match &self.actual_contract_id {
            Some(id) => format!("{}", id),
            None => "Not Set".to_string(),
        };

        write!(
            f,
            "Identity is trying to pay with the wrong token: \
expected contract ID: {}, position: {}, token ID: {}; \
actual contract ID: {}, position: {}, token ID: {}",
            expected_contract_display,
            self.expected_token_contract_position,
            self.expected_token_id,
            actual_contract_display,
            self.actual_token_contract_position,
            self.actual_token_id,
        )
    }
}

impl std::error::Error for IdentityTryingToPayWithWrongTokenError {}

impl From<IdentityTryingToPayWithWrongTokenError> for ConsensusError {
    fn from(err: IdentityTryingToPayWithWrongTokenError) -> Self {
        Self::StateError(StateError::IdentityTryingToPayWithWrongTokenError(err))
    }
}
