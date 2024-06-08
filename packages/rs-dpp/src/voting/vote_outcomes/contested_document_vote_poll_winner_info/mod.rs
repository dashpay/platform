use bincode::{Decode, Encode};
use platform_value::Identifier;

#[derive(Debug, PartialEq, Eq, Clone, Default, Encode, Decode)]
pub enum ContestedDocumentVotePollWinnerInfo {
    #[default]
    NoWinner,
    WonByIdentity(Identifier),
    Locked,
}
