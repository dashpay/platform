use platform_value::Identifier;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum ContestedDocumentVotePollWinnerInfo {
    #[default]
    NoWinner,
    WonByIdentity(Identifier),
    Locked,
}
