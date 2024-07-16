use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Default)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum YesNoAbstainVoteChoice {
    YES,
    NO,
    #[default]
    ABSTAIN,
}
