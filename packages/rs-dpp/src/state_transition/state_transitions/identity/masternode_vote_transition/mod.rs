pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::masternode_vote_transition::fields::property_names::PRO_TX_HASH;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub type MasternodeVoteTransitionLatest = MasternodeVoteTransitionV0;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.masternode_vote_state_transition"
)]
pub enum MasternodeVoteTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(MasternodeVoteTransitionV0),
}

impl MasternodeVoteTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => Ok(MasternodeVoteTransition::V0(
                MasternodeVoteTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "MasternodeVoteTransitionV0::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl OptionallyAssetLockProved for MasternodeVoteTransition {}

impl StateTransitionFieldTypes for MasternodeVoteTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![PRO_TX_HASH]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
    use crate::voting::votes::resource_vote::ResourceVote;
    use crate::voting::votes::Vote;
    use platform_value::{BinaryData, Identifier};

    fn make_vote() -> MasternodeVoteTransition {
        MasternodeVoteTransition::V0(MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::random(),
            voter_identity_id: Identifier::random(),
            vote: Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: Default::default(),
                        document_type_name: "test".to_string(),
                        index_name: "idx".to_string(),
                        index_values: vec![],
                    },
                ),
                resource_vote_choice: ResourceVoteChoice::Abstain,
            })),
            nonce: 1,
            signature_public_key_id: 2,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let t = MasternodeVoteTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            MasternodeVoteTransition::V0(_) => {}
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let t = make_vote();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored =
            MasternodeVoteTransition::deserialize_from_bytes(&bytes).expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_vote();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::MasternodeVote
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        let ids = t.modified_data_ids();
        assert_eq!(ids.len(), 1);
        let unique = t.unique_identifiers();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn test_owner_id() {
        let t = make_vote();
        match &t {
            MasternodeVoteTransition::V0(v0) => {
                assert_eq!(t.owner_id(), v0.voter_identity_id);
            }
        }
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_vote();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2]));
        assert_eq!(t.signature().as_slice(), &[1, 2]);
        t.set_signature_bytes(vec![3, 4]);
        assert_eq!(t.signature().as_slice(), &[3, 4]);
    }

    #[test]
    fn test_field_types() {
        let sig = MasternodeVoteTransition::signature_property_paths();
        assert_eq!(sig.len(), 1);
        let ids = MasternodeVoteTransition::identifiers_property_paths();
        assert_eq!(ids.len(), 1);
        let bin = MasternodeVoteTransition::binary_property_paths();
        assert!(bin.is_empty());
    }

    #[test]
    fn test_estimated_fee() {
        let t = make_vote();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert!(fee > 0);
    }

    // Legacy `StateTransitionValueConvert` round-trip and
    // unknown-version tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on
    // the outer enum derive (see `json_convertible_tests` below) — these
    // tested methods that no longer exist.

    #[test]
    fn test_into_from_v0() {
        let v0 = MasternodeVoteTransitionV0::default();
        let t: MasternodeVoteTransition = v0.clone().into();
        match t {
            MasternodeVoteTransition::V0(inner) => assert_eq!(inner, v0),
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;

    use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use crate::voting::vote_polls::VotePoll;
    use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
    use crate::voting::votes::resource_vote::ResourceVote;
    use crate::voting::votes::Vote;
    use platform_value::{platform_value, BinaryData, Identifier, Value};
    use serde_json::json;

    fn fixture_vote() -> Vote {
        Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id: Identifier::new([0x12; 32]),
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![Value::Text("dash".to_string())],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::TowardsIdentity(Identifier::new([0x34; 32])),
        }))
    }

    pub(crate) fn fixture() -> MasternodeVoteTransition {
        MasternodeVoteTransition::V0(MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::new([0x66; 32]),
            voter_identity_id: Identifier::new([0x77; 32]),
            vote: fixture_vote(),
            nonce: 99,
            signature_public_key_id: 8,
            signature: BinaryData::new(vec![0xe5; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `nonce` (u64 IdentityNonce), `signaturePublicKeyId` (u32 KeyID).
        // The `vote` field is externally tagged (`type`/`data`) at every level
        // (Vote enum, ResourceVote `$formatVersion`, VotePoll, ResourceVoteChoice).
        // The value-path assertion below uses explicit suffixes for size lock-in.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "proTxHash": Identifier::new([0x66; 32]),
                "voterIdentityId": Identifier::new([0x77; 32]),
                "vote": {
                    "$type": "resourceVote",
                    "$formatVersion": "0",
                    "votePoll": {
                        "$type": "contestedDocumentResourceVotePoll",
                        "contractId": Identifier::new([0x12; 32]),
                        "documentTypeName": "domain",
                        "indexName": "parentNameAndLabel",
                        "indexValues": ["dash"],
                    },
                    "resourceVoteChoice": {
                        "$type": "towardsIdentity",
                        "identity": Identifier::new([0x34; 32]),
                    },
                },
                "nonce": 99,
                "signaturePublicKeyId": 8,
                "signature": "5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eXl5eU=",
            })
        );
        let recovered = MasternodeVoteTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `nonce` u64 (IdentityNonce),
        // `signaturePublicKeyId` u32 (KeyID).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "proTxHash": Identifier::new([0x66; 32]),
                "voterIdentityId": Identifier::new([0x77; 32]),
                "vote": {
                    "$type": "resourceVote",
                    "$formatVersion": "0",
                    "votePoll": {
                        "$type": "contestedDocumentResourceVotePoll",
                        "contractId": Identifier::new([0x12; 32]),
                        "documentTypeName": "domain",
                        "indexName": "parentNameAndLabel",
                        "indexValues": ["dash"],
                    },
                    "resourceVoteChoice": {
                        "$type": "towardsIdentity",
                        "identity": Identifier::new([0x34; 32]),
                    },
                },
                "nonce": 99u64,
                "signaturePublicKeyId": 8u32,
                "signature": BinaryData::new(vec![0xe5; 65]),
            })
        );
        let recovered = MasternodeVoteTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
