use crate::drive::votes::paths::{
    ACTIVE_POLLS_TREE_KEY, IDENTITY_CONTENDER_POLLS_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
    RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, VOTING_STORAGE_TREE_KEY,
};
use crate::drive::votes::tree_path_storage_form::TreePathStorageForm;
use crate::drive::RootTree;
use crate::util::type_constants::DEFAULT_HASH_SIZE_USIZE;
use dpp::identifier::Identifier;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::ProtocolError;

/// What the tree path of a vote on an identity contender vote poll says about the vote: the
/// poll it is on and the choice. The poll's resource path is not in the tree, so the vote
/// resolves to the poll's unique id only.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityContenderVoteStorageForm {
    /// The unique id of the poll the vote is on
    pub vote_poll_id: Identifier,
    /// The choice: a contender's identity, or abstain
    pub resource_vote_choice: ResourceVoteChoice,
}

impl TreePathStorageForm for IdentityContenderVoteStorageForm {
    /// The path of a vote is `[votes, 'n', 'p', poll id, choice, 1, voter pro tx hash]`.
    fn try_from_tree_path(path: Vec<Vec<u8>>) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        let describe =
            |path: &[Vec<u8>]| path.iter().map(hex::encode).collect::<Vec<_>>().join("/");
        if path.len() != 7 {
            return Err(ProtocolError::VoteError(format!(
                "path {} must have 7 elements to be a vote on an identity contender vote poll",
                describe(&path)
            )));
        }
        let expected_prefix: [&[u8]; 3] = [
            &[RootTree::Votes as u8],
            &[IDENTITY_CONTENDER_POLLS_TREE_KEY as u8],
            &[ACTIVE_POLLS_TREE_KEY as u8],
        ];
        for (position, expected) in expected_prefix.iter().enumerate() {
            if path[position].as_slice() != *expected {
                return Err(ProtocolError::VoteError(format!(
                    "path {} element {} must be {} to be a vote on an identity contender vote poll, got {}",
                    describe(&path),
                    position,
                    hex::encode(expected),
                    hex::encode(&path[position])
                )));
            }
        }
        if path[5].as_slice() != [VOTING_STORAGE_TREE_KEY] {
            return Err(ProtocolError::VoteError(format!(
                "path {} sixth element must be the voting storage key",
                describe(&path)
            )));
        }
        if path[3].len() != DEFAULT_HASH_SIZE_USIZE {
            return Err(ProtocolError::VoteError(format!(
                "path {} fourth element must be a vote poll id but isn't 32 bytes long",
                describe(&path)
            )));
        }
        let vote_poll_id = Identifier::from_vec(path[3].clone())?;
        let key_vote_choice = &path[4];
        if key_vote_choice.len() != DEFAULT_HASH_SIZE_USIZE {
            return Err(ProtocolError::VoteError(format!(
                "path {} fifth element must be a contender identity id or the abstain key",
                describe(&path)
            )));
        }
        if key_vote_choice.as_slice() == RESOURCE_LOCK_VOTE_TREE_KEY_U8_32.as_slice() {
            return Err(ProtocolError::VoteError(format!(
                "path {} holds a lock vote, which identity contender vote polls do not have",
                describe(&path)
            )));
        }
        let resource_vote_choice =
            if key_vote_choice.as_slice() == RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32.as_slice() {
                ResourceVoteChoice::Abstain
            } else {
                ResourceVoteChoice::TowardsIdentity(Identifier::from_vec(key_vote_choice.clone())?)
            };
        Ok(IdentityContenderVoteStorageForm {
            vote_poll_id,
            resource_vote_choice,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::paths::vote_identity_contender_poll_choice_votes_path_vec;

    fn vote_path(choice: &ResourceVoteChoice) -> Vec<Vec<u8>> {
        let mut path = vote_identity_contender_poll_choice_votes_path_vec(&[7; 32], choice);
        path.push(vec![9; 32]);
        path
    }

    #[test]
    fn should_read_a_vote_towards_a_contender() {
        let contender = Identifier::new([3; 32]);
        let form = IdentityContenderVoteStorageForm::try_from_tree_path(vote_path(
            &ResourceVoteChoice::TowardsIdentity(contender),
        ))
        .expect("expected the storage form");
        assert_eq!(form.vote_poll_id, Identifier::new([7; 32]));
        assert_eq!(
            form.resource_vote_choice,
            ResourceVoteChoice::TowardsIdentity(contender)
        );
    }

    #[test]
    fn should_read_an_abstain_vote() {
        let form = IdentityContenderVoteStorageForm::try_from_tree_path(vote_path(
            &ResourceVoteChoice::Abstain,
        ))
        .expect("expected the storage form");
        assert_eq!(form.resource_vote_choice, ResourceVoteChoice::Abstain);
    }

    #[test]
    fn should_refuse_a_lock_vote_path() {
        let error = IdentityContenderVoteStorageForm::try_from_tree_path(vote_path(
            &ResourceVoteChoice::Lock,
        ))
        .expect_err("expected an error");
        assert!(error.to_string().contains("lock vote"), "{error}");
    }

    #[test]
    fn should_refuse_a_contested_resource_vote_path() {
        let mut path = vote_path(&ResourceVoteChoice::Abstain);
        path[1] = vec![b'c'];
        let error = IdentityContenderVoteStorageForm::try_from_tree_path(path)
            .expect_err("expected an error");
        assert!(error.to_string().contains("element 1"), "{error}");
    }

    #[test]
    fn should_refuse_a_path_of_another_length() {
        let mut path = vote_path(&ResourceVoteChoice::Abstain);
        path.pop();
        assert!(IdentityContenderVoteStorageForm::try_from_tree_path(path).is_err());
    }
}
