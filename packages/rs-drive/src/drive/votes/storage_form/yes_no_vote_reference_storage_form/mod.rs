use crate::error::drive::DriveError;
use crate::error::Error;
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

    /// Decodes the form stored in the identity votes index, a stored form that does not
    /// decode, or carries bytes past the form, being corrupted state, as the proof decoder
    /// treats it.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
        let corrupted = |reason: String| {
            Error::Drive(DriveError::CorruptedSerialization(format!(
                "serialization of yes/no vote reference {} is corrupted: {}",
                hex::encode(bytes),
                reason
            )))
        };
        let (form, consumed) =
            bincode::decode_from_slice(bytes, bincode::config::standard().with_big_endian())
                .map_err(|e| corrupted(e.to_string()))?;
        if consumed != bytes.len() {
            return Err(corrupted("it carries trailing bytes".to_string()));
        }
        Ok(form)
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

    #[test]
    fn should_refuse_trailing_bytes() {
        let form = YesNoVoteReferenceStorageForm {
            vote_choice: YesNoAbstainVoteChoice::Yes,
            identity_vote_times: 1,
        };
        let mut bytes = form.serialize().expect("serialize");
        bytes.push(0);
        assert!(matches!(
            YesNoVoteReferenceStorageForm::deserialize(&bytes),
            Err(Error::Drive(DriveError::CorruptedSerialization(_)))
        ));
    }
}
