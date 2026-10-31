use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::fee::Credits;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A decision proposal create (protocol version 15) that does not state exactly the decision
/// proposal fund under the proposal's name (11100). The fund is stated as a create's
/// prefunded voting balance, a name and an amount: the name must be `proposal`, the document
/// type's, and the amount the fee version's `decision_proposal_fund_required_amount`.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "decision proposal {document_id} must state a fund of exactly {required_fund} credits under `proposal`, it stated {}",
    stated_fund_description(stated_fund)
)]
#[platform_serialize(unversioned)]
pub struct DecisionProposalFundMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    required_fund: Credits,
    stated_fund: Option<(String, Credits)>,
}

fn stated_fund_description(stated_fund: &Option<(String, Credits)>) -> String {
    match stated_fund {
        None => "none".to_string(),
        Some((name, amount)) => format!("{amount} credits under `{name}`"),
    }
}

impl DecisionProposalFundMismatchError {
    pub fn new(
        document_id: Identifier,
        required_fund: Credits,
        stated_fund: Option<(String, Credits)>,
    ) -> Self {
        Self {
            document_id,
            required_fund,
            stated_fund,
        }
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The fund a decision proposal must state.
    pub fn required_fund(&self) -> Credits {
        self.required_fund
    }

    /// The name and amount the create stated, `None` when it stated no fund.
    pub fn stated_fund(&self) -> Option<&(String, Credits)> {
        self.stated_fund.as_ref()
    }
}

impl From<DecisionProposalFundMismatchError> for ConsensusError {
    fn from(err: DecisionProposalFundMismatchError) -> Self {
        Self::BasicError(BasicError::DecisionProposalFundMismatchError(err))
    }
}
