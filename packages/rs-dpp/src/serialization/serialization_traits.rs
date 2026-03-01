#[cfg(any(
    feature = "message-signature-verification",
    feature = "message-signing"
))]
use crate::identity::KeyType;

use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;

#[cfg(feature = "message-signature-verification")]
use crate::validation::SimpleConsensusValidationResult;
use crate::version::PlatformVersion;
#[cfg(feature = "message-signing")]
use crate::BlsModule;
use crate::ProtocolError;
use platform_value::Value;

pub trait Signable {
    fn signable_bytes(&self) -> Result<Vec<u8>, ProtocolError>;
}

pub trait PlatformSerializable {
    type Error;
    fn serialize_to_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume_to_bytes(self) -> Result<Vec<u8>, Self::Error>
    where
        Self: Sized,
    {
        self.serialize_to_bytes()
    }
}

pub trait PlatformSerializableWithPlatformVersion {
    type Error;
    /// Version based serialization is done based on the desired structure version.
    /// For example we have DataContractV0 and DataContractV1 for code based Contracts
    /// This means objects that will execute code
    /// And we would have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// which are the different ways to serialize the concept of a data contract.
    /// The data contract would call versioned_serialize. There should be a converted for each
    /// Data contract Version towards each DataContractSerializationFormat
    fn serialize_to_bytes_with_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Self::Error>;

    /// If the trait is not used just do a simple serialize
    fn serialize_consume_to_bytes_with_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Self::Error>
    where
        Self: Sized,
    {
        self.serialize_to_bytes_with_platform_version(platform_version)
    }
}

pub trait PlatformDeserializable {
    fn deserialize_from_bytes(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        Self::deserialize_from_bytes_no_limit(data)
    }

    fn deserialize_from_bytes_no_limit(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableWithPotentialValidationFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformDeserializableWithBytesLenFromVersionedStructure {
    /// We will deserialize a versioned structure into a code structure
    /// For example we have DataContractV0 and DataContractV1
    /// The system version will tell which version to deserialize into
    /// This happens by first deserializing the data into a potentially versioned structure
    /// For example we could have DataContractSerializationFormatV0 and DataContractSerializationFormatV1
    /// Both of the structures will be valid in perpetuity as they are saved into the state.
    /// So from the bytes we could get DataContractSerializationFormatV0.
    /// Then the system_version given will tell to transform DataContractSerializationFormatV0 into
    /// DataContractV1 (if system version is 1)
    fn versioned_deserialize_with_bytes_len(
        data: &[u8],
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, usize), ProtocolError>
    where
        Self: Sized;
}

pub trait PlatformLimitDeserializableFromVersionedStructure {
    fn versioned_limit_deserialize(
        data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

pub trait ValueConvertible: Serialize + DeserializeOwned {
    fn to_object(&self) -> Result<Value, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn into_object(self) -> Result<Value, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    fn from_object(value: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(value).map_err(ProtocolError::ValueError)
    }

    fn from_object_ref(value: &Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        platform_value::from_value(value.clone()).map_err(ProtocolError::ValueError)
    }
}

/// Convert to/from JSON using human-readable serde (Identifier=base58, Bytes=base64).
///
/// This trait produces clean `serde_json::Value` with native number types.
/// Any JS-boundary concerns (large number stringification) are handled by the WASM layer.
#[cfg(feature = "json-conversion")]
pub trait JsonConvertible: Serialize + DeserializeOwned {
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        serde_json::to_value(self).map_err(|e| ProtocolError::DecodingError(e.to_string()))
    }

    fn from_json(json: JsonValue) -> Result<Self, ProtocolError> {
        serde_json::from_value(json).map_err(|e| ProtocolError::DecodingError(e.to_string()))
    }
}

pub trait PlatformMessageSignable {
    #[cfg(feature = "message-signature-verification")]
    fn verify_signature(
        &self,
        public_key_type: KeyType,
        public_key_data: &[u8],
        signature: &[u8],
    ) -> SimpleConsensusValidationResult;

    #[cfg(feature = "message-signing")]
    fn sign_by_private_key(
        &self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<Vec<u8>, ProtocolError>;
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "vote-serde-conversion",
    feature = "state-transition-serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    // -----------------------------------------------------------------------
    // 1. BlockInfo JSON round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_json_round_trip() {
        use crate::block::block_info::BlockInfo;
        use crate::block::epoch::Epoch;

        let block_info = BlockInfo {
            time_ms: 1_700_000_000_000u64,
            height: 12345678u64,
            core_height: 900_000u32,
            epoch: Epoch::new(42).unwrap(),
        };

        let json = block_info.to_json().expect("to_json should succeed");

        // All numeric fields should be JSON numbers (no stringification at rs-dpp level)
        assert!(json["timeMs"].is_number());
        assert_eq!(json["timeMs"].as_u64().unwrap(), 1700000000000);

        assert!(json["height"].is_number());
        assert_eq!(json["height"].as_u64().unwrap(), 12345678);

        assert!(json["coreHeight"].is_number());
        assert_eq!(json["coreHeight"].as_u64().unwrap(), 900_000);

        // round-trip: from_json
        let restored = BlockInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(block_info, restored);
    }

    // -----------------------------------------------------------------------
    // 2. BlockInfo to_object / from_object round-trip (ValueConvertible)
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_value_round_trip() {
        use crate::block::block_info::BlockInfo;
        use crate::block::epoch::Epoch;

        let block_info = BlockInfo {
            time_ms: u64::MAX,
            height: 999u64,
            core_height: 100u32,
            epoch: Epoch::new(0).unwrap(),
        };

        let obj = block_info.to_object().expect("to_object should succeed");

        // Value should preserve native types (U64 stays U64, not string)
        let time_val = obj
            .get("timeMs")
            .expect("get should not fail on map")
            .expect("timeMs key must exist");
        assert!(
            time_val.is_integer(),
            "Value timeMs should be an integer type, got: {:?}",
            time_val
        );

        let restored = BlockInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(block_info, restored);
    }

    // -----------------------------------------------------------------------
    // 3. ExtendedEpochInfo JSON round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn extended_epoch_info_json_round_trip() {
        use crate::block::extended_epoch_info::ExtendedEpochInfo;
        use crate::block::extended_epoch_info::v0::ExtendedEpochInfoV0;

        let info = ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index: 5,
            first_block_time: 1_700_000_000_000u64,
            first_block_height: 500_000u64,
            first_core_block_height: 800_000u32,
            fee_multiplier_permille: 1_500u64,
            protocol_version: 4,
        });

        let json = info.to_json().expect("to_json should succeed");

        assert!(json["firstBlockTime"].is_number());
        assert_eq!(json["firstBlockTime"].as_u64().unwrap(), 1700000000000);

        assert!(json["firstBlockHeight"].is_number());
        assert_eq!(json["firstBlockHeight"].as_u64().unwrap(), 500000);

        assert!(json["feeMultiplierPermille"].is_number());
        assert_eq!(json["feeMultiplierPermille"].as_u64().unwrap(), 1500);

        assert!(json["firstCoreBlockHeight"].is_number());
        assert_eq!(json["firstCoreBlockHeight"].as_u64().unwrap(), 800_000);

        assert!(json["protocolVersion"].is_number());
        assert_eq!(json["protocolVersion"].as_u64().unwrap(), 4);

        // round-trip
        let restored = ExtendedEpochInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    // -----------------------------------------------------------------------
    // 4. FinalizedEpochInfo with BTreeMap<Identifier, u64>
    // -----------------------------------------------------------------------
    #[test]
    fn finalized_epoch_info_json_round_trip() {
        use crate::block::finalized_epoch_info::FinalizedEpochInfo;
        use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;

        let proposer_id = Identifier::from([1u8; 32]);
        let mut block_proposers = BTreeMap::new();
        block_proposers.insert(proposer_id, 42u64);

        let info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_700_000_000_000u64,
            first_block_height: 100_000u64,
            total_blocks_in_epoch: 2_000u64,
            first_core_block_height: 500_000u32,
            next_epoch_start_core_block_height: 500_200u32,
            total_processing_fees: 1_000_000u64,
            total_distributed_storage_fees: 500_000u64,
            total_created_storage_fees: 600_000u64,
            core_block_rewards: 10_000_000u64,
            block_proposers,
            fee_multiplier_permille: 1_000u64,
            protocol_version: 3,
        });

        let json = info.to_json().expect("to_json should succeed");

        assert!(json["firstBlockTime"].is_number());
        assert!(json["firstBlockHeight"].is_number());
        assert!(json["totalBlocksInEpoch"].is_number());
        assert!(json["totalProcessingFees"].is_number());
        assert!(json["feeMultiplierPermille"].is_number());
        assert!(json["firstCoreBlockHeight"].is_number());
        assert!(json["nextEpochStartCoreBlockHeight"].is_number());

        // blockProposers: keys should be base58 Identifier strings, values should be numbers
        let proposers = json["blockProposers"]
            .as_object()
            .expect("blockProposers should be an object");
        assert_eq!(proposers.len(), 1);

        let expected_base58 = proposer_id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert!(
            proposers.contains_key(&expected_base58),
            "Expected key {} in blockProposers, got keys: {:?}",
            expected_base58,
            proposers.keys().collect::<Vec<_>>()
        );

        let value = &proposers[&expected_base58];
        assert!(value.is_number());
        assert_eq!(value.as_u64().unwrap(), 42);

        // round-trip
        let restored = FinalizedEpochInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    // -----------------------------------------------------------------------
    // 5. ContractBounds with Identifier
    // -----------------------------------------------------------------------
    #[test]
    fn contract_bounds_single_contract_json_round_trip() {
        use crate::identity::identity_public_key::contract_bounds::ContractBounds;

        let id = Identifier::from([0xABu8; 32]);
        let bounds = ContractBounds::SingleContract { id };

        let json = bounds.to_json().expect("to_json should succeed");

        // The id field should be a base58 string
        assert!(
            json["id"].is_string(),
            "Identifier should be a base58 string, got: {:?}",
            json["id"]
        );

        let expected_base58 = id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert_eq!(json["id"].as_str().unwrap(), expected_base58);

        // round-trip
        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    #[test]
    fn contract_bounds_document_type_json_round_trip() {
        use crate::identity::identity_public_key::contract_bounds::ContractBounds;

        let id = Identifier::from([0xCDu8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "myDocument".to_string(),
        };

        let json = bounds.to_json().expect("to_json should succeed");

        assert!(json["id"].is_string());
        assert_eq!(
            json["documentTypeName"].as_str().unwrap(),
            "myDocument"
        );

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    // -----------------------------------------------------------------------
    // 6. ResourceVoteChoice with Identifier
    // -----------------------------------------------------------------------
    #[test]
    fn resource_vote_choice_towards_identity_json_round_trip() {
        use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

        let id = Identifier::from([0x42u8; 32]);
        let choice = ResourceVoteChoice::TowardsIdentity(id);

        let json = choice.to_json().expect("to_json should succeed");

        // Should serialize the identifier
        let json_str = serde_json::to_string(&json).unwrap();
        let expected_base58 = id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert!(
            json_str.contains(&expected_base58),
            "JSON should contain base58 identifier {}, got: {}",
            expected_base58,
            json_str
        );

        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }

    #[test]
    fn resource_vote_choice_abstain_json_round_trip() {
        use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

        let choice = ResourceVoteChoice::Abstain;
        let json = choice.to_json().expect("to_json should succeed");
        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }

    #[test]
    fn resource_vote_choice_lock_json_round_trip() {
        use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;

        let choice = ResourceVoteChoice::Lock;
        let json = choice.to_json().expect("to_json should succeed");
        let restored = ResourceVoteChoice::from_json(json).expect("from_json should succeed");
        assert_eq!(choice, restored);
    }

    // -----------------------------------------------------------------------
    // 7. Vote versioned enum JSON round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn vote_json_round_trip() {
        use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use crate::voting::vote_polls::VotePoll;
        use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
        use crate::voting::votes::resource_vote::ResourceVote;
        use crate::voting::votes::Vote;

        let contract_id = Identifier::from([0x11u8; 32]);
        let towards_id = Identifier::from([0x22u8; 32]);

        let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    contract_id,
                    document_type_name: "domain".to_string(),
                    index_name: "parentNameAndLabel".to_string(),
                    index_values: vec![
                        platform_value::Value::Text("dash".to_string()),
                    ],
                },
            ),
            resource_vote_choice: ResourceVoteChoice::TowardsIdentity(towards_id),
        }));

        let json = vote.to_json().expect("to_json should succeed");

        // Verify it's a valid JSON object
        assert!(json.is_object(), "Vote JSON should be an object");

        // round-trip
        let restored = Vote::from_json(json).expect("from_json should succeed");
        assert_eq!(vote, restored);
    }

    // -----------------------------------------------------------------------
    // 8. ChangeControlRules round-trip
    // -----------------------------------------------------------------------
    #[test]
    fn change_control_rules_json_round_trip() {
        use crate::data_contract::change_control_rules::ChangeControlRules;
        use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;

        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: true,
        });

        let json = rules.to_json().expect("to_json should succeed");

        // Verify boolean fields
        assert_eq!(
            json["changingAuthorizedActionTakersToNoOneAllowed"]
                .as_bool()
                .unwrap(),
            true
        );
        assert_eq!(
            json["changingAdminActionTakersToNoOneAllowed"]
                .as_bool()
                .unwrap(),
            false
        );
        assert_eq!(
            json["selfChangingAdminActionTakersAllowed"]
                .as_bool()
                .unwrap(),
            true
        );

        // round-trip
        let restored = ChangeControlRules::from_json(json).expect("from_json should succeed");
        assert_eq!(rules, restored);
    }

    #[test]
    fn change_control_rules_with_group_json_round_trip() {
        use crate::data_contract::change_control_rules::ChangeControlRules;
        use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;

        let rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::Group(3),
            admin_action_takers: AuthorizedActionTakers::Identity(Identifier::from([0xFFu8; 32])),
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        });

        let json = rules.to_json().expect("to_json should succeed");
        let restored = ChangeControlRules::from_json(json).expect("from_json should succeed");
        assert_eq!(rules, restored);
    }

    // -----------------------------------------------------------------------
    // 9. TokenConfiguration with base_supply u64
    // -----------------------------------------------------------------------
    #[test]
    fn token_configuration_json_round_trip() {
        use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
        use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;

        let config = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());

        let json = config.to_json().expect("to_json should succeed");

        assert!(json["baseSupply"].is_number());
        assert_eq!(json["baseSupply"].as_u64().unwrap(), 100000);

        // round-trip
        let restored =
            TokenConfiguration::from_json(json).expect("from_json should succeed");
        assert_eq!(config, restored);
    }

    #[test]
    fn token_configuration_large_supply_json_round_trip() {
        use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
        use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;

        let mut config = TokenConfigurationV0::default_most_restrictive();
        config.base_supply = u64::MAX;
        let config = TokenConfiguration::V0(config);

        let json = config.to_json().expect("to_json should succeed");

        assert!(json["baseSupply"].is_number());
        assert_eq!(json["baseSupply"].as_u64().unwrap(), u64::MAX);

        let restored = TokenConfiguration::from_json(json).expect("from_json should succeed");
        assert_eq!(config, restored);
    }

    // -----------------------------------------------------------------------
    // 10. IdentityTokenInfo (versioned enum, feature-gated)
    // -----------------------------------------------------------------------
    #[test]
    fn identity_token_info_json_round_trip() {
        use crate::tokens::info::v0::IdentityTokenInfoV0;
        use crate::tokens::info::IdentityTokenInfo;

        let info = IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true });

        let json = info.to_json().expect("to_json should succeed");

        // Verify the version tag
        assert_eq!(
            json["$formatVersion"].as_str().unwrap(),
            "0",
            "Version tag should be '0'"
        );

        // Verify the boolean field
        assert_eq!(json["frozen"].as_bool().unwrap(), true);

        // round-trip
        let restored = IdentityTokenInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    #[test]
    fn identity_token_info_unfrozen_json_round_trip() {
        use crate::tokens::info::v0::IdentityTokenInfoV0;
        use crate::tokens::info::IdentityTokenInfo;

        let info = IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: false });

        let json = info.to_json().expect("to_json should succeed");
        let restored = IdentityTokenInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(info, restored);
    }

    // -----------------------------------------------------------------------
    // Bonus: TokenStatus (versioned enum)
    // -----------------------------------------------------------------------
    #[test]
    fn token_status_json_round_trip() {
        use crate::tokens::status::v0::TokenStatusV0;
        use crate::tokens::status::TokenStatus;

        let status = TokenStatus::V0(TokenStatusV0 { paused: true });

        let json = status.to_json().expect("to_json should succeed");

        assert_eq!(json["$formatVersion"].as_str().unwrap(), "0");
        assert_eq!(json["paused"].as_bool().unwrap(), true);

        let restored = TokenStatus::from_json(json).expect("from_json should succeed");
        assert_eq!(status, restored);
    }

    // -----------------------------------------------------------------------
    // Bonus: BlockInfo into_object consumes self
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_into_object_round_trip() {
        use crate::block::block_info::BlockInfo;
        use crate::block::epoch::Epoch;

        let block_info = BlockInfo {
            time_ms: 42u64,
            height: 100u64,
            core_height: 50u32,
            epoch: Epoch::new(1).unwrap(),
        };

        let expected = block_info;
        let obj = block_info.into_object().expect("into_object should succeed");
        let restored = BlockInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(expected, restored);
    }

    // -----------------------------------------------------------------------
    // Bonus: BlockInfo from_object_ref
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_from_object_ref() {
        use crate::block::block_info::BlockInfo;
        use crate::block::epoch::Epoch;

        let block_info = BlockInfo {
            time_ms: 1_000u64,
            height: 200u64,
            core_height: 10u32,
            epoch: Epoch::new(3).unwrap(),
        };

        let obj = block_info.to_object().expect("to_object should succeed");

        // from_object_ref takes a reference and should produce the same result
        let restored1 = BlockInfo::from_object_ref(&obj).expect("from_object_ref should succeed");
        let restored2 = BlockInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(restored1, restored2);
        assert_eq!(block_info, restored1);
    }

    // -----------------------------------------------------------------------
    // Bonus: ExtendedEpochInfo ValueConvertible
    // -----------------------------------------------------------------------
    #[test]
    fn extended_epoch_info_value_round_trip() {
        use crate::block::extended_epoch_info::ExtendedEpochInfo;
        use crate::block::extended_epoch_info::v0::ExtendedEpochInfoV0;

        let info = ExtendedEpochInfo::V0(ExtendedEpochInfoV0 {
            index: 10,
            first_block_time: u64::MAX,
            first_block_height: 0,
            first_core_block_height: u32::MAX,
            fee_multiplier_permille: 1_000,
            protocol_version: 1,
        });

        let obj = info.to_object().expect("to_object should succeed");
        let restored =
            ExtendedEpochInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(info, restored);
    }

    // -----------------------------------------------------------------------
    // Bonus: FinalizedEpochInfo ValueConvertible (no Identifier map keys)
    // -----------------------------------------------------------------------
    #[test]
    fn finalized_epoch_info_value_round_trip_empty_proposers() {
        use crate::block::finalized_epoch_info::FinalizedEpochInfo;
        use crate::block::finalized_epoch_info::v0::FinalizedEpochInfoV0;

        // Use empty block_proposers to avoid the Identifier-as-map-key
        // serialization asymmetry (to_value serializes Identifier as base58
        // string key, but from_value expects bytes)
        let info = FinalizedEpochInfo::V0(FinalizedEpochInfoV0 {
            first_block_time: 1_000_000u64,
            first_block_height: 10_000u64,
            total_blocks_in_epoch: 500u64,
            first_core_block_height: 50_000u32,
            next_epoch_start_core_block_height: 50_200u32,
            total_processing_fees: 100u64,
            total_distributed_storage_fees: 50u64,
            total_created_storage_fees: 60u64,
            core_block_rewards: 1_000u64,
            block_proposers: BTreeMap::new(),
            fee_multiplier_permille: 1_000u64,
            protocol_version: 2,
        });

        let obj = info.to_object().expect("to_object should succeed");
        let restored =
            FinalizedEpochInfo::from_object(obj).expect("from_object should succeed");
        assert_eq!(info, restored);
    }

    // -----------------------------------------------------------------------
    // Edge case: BlockInfo with zero values
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_zero_values_json_round_trip() {
        use crate::block::block_info::BlockInfo;

        let block_info = BlockInfo::default();

        let json = block_info.to_json().expect("to_json should succeed");

        assert!(json["timeMs"].is_number());
        assert_eq!(json["timeMs"].as_u64().unwrap(), 0);

        assert!(json["height"].is_number());
        assert_eq!(json["height"].as_u64().unwrap(), 0);

        let restored = BlockInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(block_info, restored);
    }

    // -----------------------------------------------------------------------
    // Edge case: BlockInfo with u64::MAX
    // -----------------------------------------------------------------------
    #[test]
    fn block_info_max_u64_json_round_trip() {
        use crate::block::block_info::BlockInfo;
        use crate::block::epoch::Epoch;

        let block_info = BlockInfo {
            time_ms: u64::MAX,
            height: u64::MAX,
            core_height: u32::MAX,
            epoch: Epoch::new(100).unwrap(),
        };

        let json = block_info.to_json().expect("to_json should succeed");

        assert!(json["timeMs"].is_number());
        assert_eq!(json["timeMs"].as_u64().unwrap(), u64::MAX);
        assert_eq!(json["height"].as_u64().unwrap(), u64::MAX);

        let restored = BlockInfo::from_json(json).expect("from_json should succeed");
        assert_eq!(block_info, restored);
    }

    // -----------------------------------------------------------------------
    // ContractBounds ValueConvertible
    // -----------------------------------------------------------------------
    // -----------------------------------------------------------------------
    // ChainAssetLockProof JSON round-trip (OutPoint human-readable)
    // -----------------------------------------------------------------------
    #[test]
    fn chain_asset_lock_proof_json_round_trip() {
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dashcore::{OutPoint, Txid};
        use std::str::FromStr;

        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let json = proof.to_json().expect("to_json should succeed");

        // OutPoint should be "txid:vout" string (human-readable serde_json)
        assert!(
            json["outPoint"].is_string(),
            "outPoint should be a string, got: {:?}",
            json["outPoint"]
        );
        assert!(
            json["outPoint"].as_str().unwrap().contains(":"),
            "outPoint should contain ':'"
        );
        assert_eq!(json["coreChainLockedHeight"].as_u64().unwrap(), 11);

        let restored = ChainAssetLockProof::from_json(json).expect("from_json should succeed");
        assert_eq!(proof, restored);
    }

    #[test]
    fn chain_asset_lock_proof_value_round_trip() {
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dashcore::{OutPoint, Txid};
        use std::str::FromStr;

        let txid_hex = "e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d";
        let txid = Txid::from_str(txid_hex).unwrap();
        let proof = ChainAssetLockProof {
            core_chain_locked_height: 11,
            out_point: OutPoint { txid, vout: 1 },
        };

        let obj = proof.to_object().expect("to_object should succeed");
        let restored = ChainAssetLockProof::from_object(obj).expect("from_object should succeed");
        assert_eq!(proof, restored);
    }

    // -----------------------------------------------------------------------
    // ContractBounds ValueConvertible
    // -----------------------------------------------------------------------
    #[test]
    fn contract_bounds_value_round_trip() {
        use crate::identity::identity_public_key::contract_bounds::ContractBounds;

        let id = Identifier::from([0x55u8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "note".to_string(),
        };

        let obj = bounds.to_object().expect("to_object should succeed");
        let restored = ContractBounds::from_object(obj).expect("from_object should succeed");
        assert_eq!(bounds, restored);
    }
}
