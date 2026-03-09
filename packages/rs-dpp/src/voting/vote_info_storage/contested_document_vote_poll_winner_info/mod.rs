use crate::serialization::ValueConvertible;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Encode, Decode, Serialize, Deserialize, ValueConvertible)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ContestedDocumentVotePollWinnerInfo {
    #[default]
    NoWinner,
    WonByIdentity(Identifier),
    Locked,
}

impl fmt::Display for ContestedDocumentVotePollWinnerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContestedDocumentVotePollWinnerInfo::NoWinner => write!(f, "NoWinner"),
            ContestedDocumentVotePollWinnerInfo::WonByIdentity(identifier) => {
                write!(f, "WonByIdentity({})", identifier)
            }
            ContestedDocumentVotePollWinnerInfo::Locked => write!(f, "Locked"),
        }
    }
}

// Manual impl because ContestedDocumentVotePollWinnerInfo is a flat enum
// (not versioned V0/V1).
#[cfg(feature = "json-conversion")]
impl JsonConvertible for ContestedDocumentVotePollWinnerInfo {}
