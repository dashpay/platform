mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::identity::KeyID;

use crate::prelude::{Identifier, IdentityNonce};

use crate::protocol_error::ProtocolError;
use crate::voting::votes::Vote;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
#[derive(Default)]
pub struct MasternodeVoteTransitionV0 {
    // Own ST fields
    pub pro_tx_hash: Identifier,
    pub voter_identity_id: Identifier,
    pub vote: Vote,
    pub nonce: IdentityNonce,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
    use crate::voting::votes::resource_vote::ResourceVote;
    use crate::voting::votes::Vote;
    use platform_value::Identifier;
    use rand::Rng;
    use std::fmt::Debug;

    fn test_masternode_vote_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
        transition: T,
    ) where
        <T as PlatformSerializable>::Error: std::fmt::Debug,
    {
        let serialized = T::serialize_to_bytes(&transition).expect("expected to serialize");
        let deserialized =
            T::deserialize_from_bytes(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_masternode_vote_transition1() {
        let mut rng = rand::thread_rng();
        let transition = MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::random(),
            voter_identity_id: Identifier::random(),
            vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: Default::default(),
                        document_type_name: "hello".to_string(),
                        index_name: "index_1".to_string(),
                        index_values: vec![],
                    },
                ),
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(Identifier::random()),
            })),
            nonce: 1,
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_masternode_vote_transition(transition);
    }
}
