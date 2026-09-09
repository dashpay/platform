mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::identity::KeyID;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;

use crate::prelude::{Identifier, IdentityNonce};

use crate::protocol_error::ProtocolError;
use crate::voting::votes::Vote;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
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
    feature = "serde-conversion",
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

    fn make_vote_v0() -> MasternodeVoteTransitionV0 {
        MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::random(),
            voter_identity_id: Identifier::random(),
            vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: Default::default(),
                        document_type_name: "test_doc".to_string(),
                        index_name: "idx".to_string(),
                        index_values: vec![],
                    },
                ),
                resource_vote_choice: ResourceVoteChoice::Abstain,
            })),
            nonce: 7,
            signature_public_key_id: 3,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn test_default() {
        let t = MasternodeVoteTransitionV0::default();
        assert_eq!(t.nonce, 0);
        assert_eq!(t.signature_public_key_id, 0);
    }

    #[test]
    fn test_state_transition_like_v0() {
        use crate::state_transition::{
            StateTransitionLike, StateTransitionOwned, StateTransitionType,
        };
        let t = make_vote_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::MasternodeVote
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.voter_identity_id]);
        assert_eq!(t.owner_id(), t.voter_identity_id);
    }

    #[test]
    fn test_unique_identifiers_v0() {
        use crate::state_transition::StateTransitionLike;
        let t = make_vote_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_identity_signed_v0() {
        use crate::identity::{Purpose, SecurityLevel};
        use crate::state_transition::StateTransitionIdentitySigned;
        let mut t = make_vote_v0();
        assert_eq!(t.signature_public_key_id(), 3);
        t.set_signature_public_key_id(77);
        assert_eq!(t.signature_public_key_id(), 77);
        let security = t.security_level_requirement(Purpose::VOTING);
        assert!(security.contains(&SecurityLevel::CRITICAL));
        assert!(security.contains(&SecurityLevel::HIGH));
        assert!(security.contains(&SecurityLevel::MEDIUM));
        let purpose = t.purpose_requirement();
        assert_eq!(purpose, vec![Purpose::VOTING]);
    }

    #[test]
    fn test_single_signed_v0() {
        use crate::state_transition::StateTransitionSingleSigned;
        use platform_value::BinaryData;
        let mut t = make_vote_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![9, 8, 7]));
        assert_eq!(t.signature().as_slice(), &[9, 8, 7]);
        t.set_signature_bytes(vec![6, 5]);
        assert_eq!(t.signature().as_slice(), &[6, 5]);
    }

    #[test]
    fn test_into_state_transition_v0() {
        use crate::state_transition::StateTransition;
        let t = make_vote_v0();
        let st: StateTransition = t.into();
        match st {
            StateTransition::MasternodeVote(_) => {}
            _ => panic!("expected MasternodeVote"),
        }
    }

    // Legacy `StateTransitionValueConvert` round-trip / cleaned-object /
    // skip-signature tests on the V0 inner struct deleted in Phase D
    // step 9. The canonical `JsonConvertible` / `ValueConvertible`
    // round-trip is exercised on the outer enum derive — these tested
    // methods that no longer exist.
}
