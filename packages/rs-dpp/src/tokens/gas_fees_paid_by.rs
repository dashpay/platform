use bincode_derive::{Decode, Encode};
use derive_more::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, Default, PartialEq, Display)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum GasFeesPaidBy {
    /// The user pays the gas fees
    #[default]
    DocumentOwner,
    /// The contract owner pays the gas fees
    ContractOwner,
    /// The user is stating his willingness to pay the gas fee if the Contract owner's balance is
    /// insufficient.
    PreferContractOwner,
}