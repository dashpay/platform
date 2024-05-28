use bincode::{Decode, Encode};
use platform_value::Identifier;
#[cfg(feature = "vote-serde-conversion")]
use serde::{Deserialize, Serialize};

/// A resource votes is a votes determining what we should do with a contested resource.
/// For example Alice and Bob both want the username "Malaka"
/// Some would votes for Alice to get it by putting in her Identifier.
/// Some would votes for Bob to get it by putting in Bob's Identifier.
/// Let's say someone voted, but is now not quite sure of their votes, they can abstain.
/// Lock is there to signal that the shared resource should be given to no one.
/// In this case Malaka might have a bad connotation in Greek, hence some might votes to Lock
/// the name.
///
#[derive(Debug, Clone, Copy, Encode, Decode, PartialEq, Default)]
#[cfg_attr(
    feature = "vote-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum ResourceVoteChoice {
    TowardsIdentity(Identifier),
    #[default]
    Abstain,
    Defer,
    Lock,
}

impl ResourceVoteChoice {
    pub fn to_key(&self) -> Vec<u8> {
        match self {
            ResourceVoteChoice::TowardsIdentity(identity_id) => identity_id.to_vec(),
            ResourceVoteChoice::Abstain => vec![128],
            ResourceVoteChoice::Defer => vec![129],
            ResourceVoteChoice::Lock => vec![130],
        }
    }
}
