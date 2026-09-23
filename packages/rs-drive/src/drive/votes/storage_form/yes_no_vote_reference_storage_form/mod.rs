use bincode::{Decode, DecodeUntrusted, Encode};
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;

/// What the identity votes index of the decisions branch stores for one masternode's vote on
/// one yes/no poll: the answer, and how many times the masternode voted on the poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub struct YesNoVoteReferenceStorageForm {
    /// The masternode's current answer
    pub vote_choice: YesNoAbstainVoteChoice,
    /// How many times the masternode voted on the poll, counting this vote
    pub identity_vote_times: u16,
}

impl YesNoVoteReferenceStorageForm {
    /// Encodes the form for storage.
    pub fn serialize(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::encode_to_vec(self, bincode::config::standard().with_big_endian())
    }

    /// Decodes a stored form.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        bincode::decode_from_slice(bytes, bincode::config::standard().with_big_endian())
            .map(|(form, _)| form)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_round_trip() {
        let form = YesNoVoteReferenceStorageForm {
            vote_choice: YesNoAbstainVoteChoice::No,
            identity_vote_times: 3,
        };
        let bytes = form.serialize().expect("serialize");
        assert_eq!(
            YesNoVoteReferenceStorageForm::deserialize(&bytes).expect("deserialize"),
            form
        );
    }
}
