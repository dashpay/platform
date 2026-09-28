use crate::data_contract::DataContractFactory;
use crate::prelude::*;
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

use crate::data_contract::accessors::v0::DataContractV0Setters;
use crate::data_contract::config::v1::DataContractConfigSettersV1;
use crate::data_contract::config::DataContractConfig;
pub use data_contracts::*;
use platform_version::version::PlatformVersion;

pub trait ConfigurationForSystemContract {
    fn configuration_in_platform_version(
        &self,
        version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError>;
}

impl ConfigurationForSystemContract for SystemDataContract {
    fn configuration_in_platform_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match self {
            SystemDataContract::Withdrawals => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::MasternodeRewards => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            // Reserved slot with no implementation. Any caller that reaches here
            // has a bug (they should have short-circuited on `source()` returning
            // `ContractReserved`). Return a harmless default config rather than
            // panicking so this failure mode stays non-fatal.
            SystemDataContract::FeatureFlags => {
                DataContractConfig::default_for_version(platform_version)
            }
            SystemDataContract::DPNS => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::Dashpay => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::WalletUtils => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(false);
                Ok(config)
            }
            SystemDataContract::TokenHistory => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::KeywordSearch => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::DocumentHistory => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::AppConnect => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
            SystemDataContract::ModerationCharters => {
                let mut config = DataContractConfig::default_for_version(platform_version)?;
                config.set_sized_integer_types_enabled(true);
                Ok(config)
            }
        }
    }
}

/// Builds a system contract from its source, with full validation, under its published id.
///
/// The contract is parsed under the published id rather than one derived from the owner and
/// a nonce and renamed afterwards: the document types remember the contract id they belong
/// to, and the checks that compare against it read that one. The moderation charters
/// contract's key references require keys bound to its own document types (`boundTo`), which
/// would name the wrong contract under any other id.
fn create_data_contract(
    factory: &DataContractFactory,
    system_contract: SystemDataContract,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let DataContractSource {
        id_bytes,
        owner_id_bytes,
        version,
        definitions,
        document_schemas,
    } = system_contract
        .source(platform_version)
        .map_err(|e| ProtocolError::Generic(e.to_string()))?;

    let mut data_contract = factory.create_with_id(
        Identifier::from(id_bytes),
        Identifier::from(owner_id_bytes),
        0,
        document_schemas.into(),
        Some(system_contract.configuration_in_platform_version(platform_version)?),
        definitions.map(|def| def.into()),
    )?;

    data_contract.data_contract_mut().set_version(version);

    Ok(data_contract.data_contract_owned())
}

pub fn load_system_data_contract(
    system_contract: SystemDataContract,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let factory = DataContractFactory::new(platform_version.protocol_version)?;

    create_data_contract(&factory, system_contract, platform_version)
}

pub fn load_system_data_contracts(
    system_contracts: BTreeSet<SystemDataContract>,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<SystemDataContract, DataContract>, ProtocolError> {
    let factory = DataContractFactory::new(platform_version.protocol_version)?;

    system_contracts
        .into_iter()
        .map(|system_contract| {
            let data_contract = create_data_contract(&factory, system_contract, platform_version)?;

            Ok((system_contract, data_contract))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use platform_version::TryIntoPlatformVersioned;
    #[test]
    fn test_load_system_data_contract_v8_vs_v9() {
        let contract_1 = load_system_data_contract(
            SystemDataContract::TokenHistory,
            PlatformVersion::get(8).unwrap(),
        )
        .expect("data_contract");
        let contract_2 = load_system_data_contract(
            SystemDataContract::TokenHistory,
            PlatformVersion::get(9).unwrap(),
        )
        .expect("data_contract");
        assert_ne!(contract_1, contract_2);
    }

    #[test]
    fn serialize_withdrawal_contract_v1_vs_v9() {
        let contract_1 = load_system_data_contract(
            SystemDataContract::Withdrawals,
            PlatformVersion::get(1).unwrap(),
        )
        .expect("data_contract");
        let contract_2 = load_system_data_contract(
            SystemDataContract::Withdrawals,
            PlatformVersion::get(9).unwrap(),
        )
        .expect("data_contract");

        assert_ne!(contract_1, contract_2);
        let v1_ser: DataContractInSerializationFormat = contract_1
            .clone()
            .try_into_platform_versioned(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v2_ser: DataContractInSerializationFormat = contract_2
            .clone()
            .try_into_platform_versioned(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_ser, v2_ser);

        let v1_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(1).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_1
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1757); // this will still use a config v0 without sized_integer_types

        let v1_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v8_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(8).unwrap())
            .expect("expected to serialize");
        let v9_bytes = contract_2
            .serialize_to_bytes_with_platform_version(PlatformVersion::get(9).unwrap())
            .expect("expected to serialize");
        assert_eq!(v1_bytes.len(), 1747);
        assert_eq!(v8_bytes.len(), 1747);
        assert_eq!(v9_bytes.len(), 1758); // this will use a config v1 in serialization with sized_integer_types
    }
}

#[cfg(all(test, feature = "app-connect-contract", feature = "validation"))]
mod app_connect_tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use crate::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use platform_value::Value;

    fn response(contract: &DataContract) -> Document {
        let mut document = contract
            .document_type_for_name("loginKeyResponse")
            .expect("response type")
            .random_document(Some(42), PlatformVersion::latest())
            .expect("response document");
        document.set_properties(BTreeMap::from([
            (
                "appEphemeralPubKeyHash".into(),
                Value::Bytes(vec![0x11; 20]),
            ),
            ("walletEphemeralPubKey".into(), Value::Bytes(vec![0x22; 33])),
            ("encryptedPayload".into(), Value::Bytes(vec![0x33; 60])),
        ]));
        document
    }

    #[test]
    fn should_validate_app_connect_response_lengths() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::AppConnect, platform_version)
            .expect("system contract");
        for (property, length, expected) in [
            ("appEphemeralPubKeyHash", 19, false),
            ("appEphemeralPubKeyHash", 20, true),
            ("appEphemeralPubKeyHash", 21, false),
            ("walletEphemeralPubKey", 32, false),
            ("walletEphemeralPubKey", 33, true),
            ("walletEphemeralPubKey", 34, false),
            ("encryptedPayload", 59, false),
            ("encryptedPayload", 60, true),
            ("encryptedPayload", 572, true),
            ("encryptedPayload", 573, false),
        ] {
            let mut document = response(&contract);
            document.set(property, Value::Bytes(vec![0x44; length]));
            let result = contract
                .validate_document("loginKeyResponse", &document, platform_version)
                .expect("validation executes");
            assert_eq!(
                result.is_valid(),
                expected,
                "{property}: {length} bytes: {result:?}"
            );
        }
    }

    #[test]
    fn should_require_only_the_three_app_connect_response_properties() {
        let platform_version = PlatformVersion::latest();
        let contract = load_system_data_contract(SystemDataContract::AppConnect, platform_version)
            .expect("system contract");
        assert_eq!(contract.document_types().len(), 1);
        for property in [
            "appEphemeralPubKeyHash",
            "walletEphemeralPubKey",
            "encryptedPayload",
        ] {
            let mut document = response(&contract);
            document.properties_mut().remove(property);
            assert!(
                !contract
                    .validate_document("loginKeyResponse", &document, platform_version)
                    .expect("validation executes")
                    .is_valid(),
                "{property} is required"
            );
        }
        let mut document = response(&contract);
        document.set("contractId", Value::Bytes(vec![0x55; 32]));
        assert!(
            !contract
                .validate_document("loginKeyResponse", &document, platform_version)
                .expect("validation executes")
                .is_valid(),
            "the former contractId field is not admitted"
        );
    }
}

#[cfg(all(test, feature = "moderation-charters-contract", feature = "validation"))]
mod moderation_charters_tests {
    use super::*;
    use crate::consensus::basic::document::PropertyConstraintViolation;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::accessors::{
        DocumentTypeV0Getters, DocumentTypeV2Getters,
    };
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::data_contract::document_type::{
        ContestedIndexResolution, ContractReferenceModeration, DistinctFrom,
        DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentType, EncryptedForRecipient,
        EncryptionScheme, KeyReferenceIdentityProperty, LookupKeySource, PropertyReference,
        StringPropertySizes,
    };
    use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use crate::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use crate::identity::Purpose;
    use crate::moderation_charter::{
        property_names, ElectedCharter, ModerationCharterRewardSplit, SubmittedCharter,
        ADDED_MODERATOR_DOCUMENT_TYPE_NAME, ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
        JOIN_REQUEST_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
        REASON_DOCUMENT_TYPE_NAME, REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
        RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
    };
    use platform_value::{Identifier, Value};

    fn contract() -> DataContract {
        load_system_data_contract(
            SystemDataContract::ModerationCharters,
            PlatformVersion::latest(),
        )
        .expect("the moderation charters contract loads")
    }

    fn document_type<'a>(contract: &'a DataContract, name: &str) -> &'a DocumentType {
        contract
            .document_types()
            .get(name)
            .unwrap_or_else(|| panic!("the {name} type"))
    }

    fn reference<'a>(
        contract: &'a DataContract,
        type_name: &str,
        property: &str,
    ) -> PropertyReference<'a> {
        document_type(contract, type_name)
            .flattened_properties()
            .get(property)
            .unwrap_or_else(|| panic!("{type_name}.{property}"))
            .property_type
            .reference()
            .unwrap_or_else(|| panic!("{type_name}.{property} declares a reference"))
    }

    fn proposal() -> SubmittedCharter {
        SubmittedCharter {
            target_contract_id: Identifier::from([9u8; 32]),
            description: "We remove spam and doxing within a day.".to_string(),
            reasons: vec![Identifier::from([3u8; 32]), Identifier::from([4u8; 32])],
            moderators_share: None,
            reward_split: ModerationCharterRewardSplit {
                leader: 10,
                equal: 40,
                actions: 50,
            },
        }
    }

    fn document_with(
        contract: &DataContract,
        type_name: &str,
        properties: std::collections::BTreeMap<String, Value>,
    ) -> Document {
        let mut document = document_type(contract, type_name)
            .random_document(Some(42), PlatformVersion::latest())
            .expect("a random document");
        document.set_properties(properties);
        document
    }

    fn schema_validation(
        contract: &DataContract,
        type_name: &str,
        document: &Document,
    ) -> Vec<ConsensusError> {
        contract
            .validate_document(type_name, document, PlatformVersion::latest())
            .expect("validation executes")
            .errors
    }

    #[test]
    fn should_spell_the_same_id_in_the_crate_and_in_dpp() {
        assert_eq!(
            moderation_charters_contract::ID,
            MODERATION_CHARTERS_CONTRACT_ID
        );
        assert_eq!(contract().id(), MODERATION_CHARTERS_CONTRACT_ID);
    }

    /// The document types remember the contract id they were parsed under, and the key bound
    /// and lookup checks compare against it, so the contract has to be built under its
    /// published id rather than renamed afterwards.
    #[test]
    fn should_parse_every_document_type_under_the_published_id() {
        let contract = contract();
        let mut names: Vec<&str> = contract
            .document_types()
            .keys()
            .map(String::as_str)
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                JOIN_REQUEST_DOCUMENT_TYPE_NAME,
                REASON_DOCUMENT_TYPE_NAME,
                REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            ]
        );
        for name in names {
            let document_type = document_type(&contract, name);
            assert_eq!(
                document_type.data_contract_id(),
                MODERATION_CHARTERS_CONTRACT_ID
            );
            assert!(!document_type.documents_mutable(), "{name} is immutable");
            // A team change is undone by deleting it: an addition takes the member off, a
            // removal puts them back, a resignation request is withdrawn. What makes the
            // charter is final.
            assert_eq!(
                document_type.documents_can_be_deleted(),
                [
                    ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                    REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                    RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
                ]
                .contains(&name),
                "{name}: only the team changes can be deleted"
            );
        }
    }

    #[test]
    fn should_declare_only_the_elected_charter_as_a_no_locking_contest() {
        let contract = contract();
        for name in [
            REASON_DOCUMENT_TYPE_NAME,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            JOIN_REQUEST_DOCUMENT_TYPE_NAME,
            ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
            REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
            RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
        ] {
            assert!(
                document_type(&contract, name)
                    .find_contested_index()
                    .is_none(),
                "{name} is not contested"
            );
        }
        let index = document_type(&contract, ELECTED_CHARTER_DOCUMENT_TYPE_NAME)
            .find_contested_index()
            .expect("the elected charter type has a contested index");
        assert_eq!(index.name, "byTargetContract");
        assert!(index.unique);
        assert_eq!(
            index
                .properties
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec![property_names::TARGET_CONTRACT_ID]
        );
        assert_eq!(
            index
                .contested_index
                .as_ref()
                .expect("contested")
                .resolution,
            ContestedIndexResolution::MasternodeVoteNoLocking
        );
    }

    #[test]
    fn should_let_teams_form_before_the_election_opens() {
        let contract = contract();
        let moderation =
            |type_name| match reference(&contract, type_name, property_names::TARGET_CONTRACT_ID) {
                PropertyReference::Value(DocumentPropertyReferenceTarget::Contract {
                    contract_requirements,
                }) => contract_requirements.moderation,
                other => panic!("{type_name}.targetContractId: {other:?}"),
            };
        assert_eq!(
            moderation(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME),
            Some(ContractReferenceModeration::Elected)
        );
        assert_eq!(
            moderation(ELECTED_CHARTER_DOCUMENT_TYPE_NAME),
            Some(ContractReferenceModeration::ElectionOpen)
        );
    }

    #[test]
    fn should_list_reasons_as_references_to_reason_documents() {
        let contract = contract();
        match reference(
            &contract,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            property_names::REASONS,
        ) {
            PropertyReference::Elements {
                target:
                    DocumentPropertyReferenceTarget::PermanentDocument {
                        contract_id: None,
                        document_type_name,
                        ..
                    },
                max_items,
            } => {
                assert_eq!(document_type_name, REASON_DOCUMENT_TYPE_NAME);
                assert_eq!(max_items, 64);
            }
            other => panic!("reasons: {other:?}"),
        }
    }

    #[test]
    fn should_address_a_join_request_to_the_leader_under_bound_keys() {
        let contract = contract();
        match reference(
            &contract,
            JOIN_REQUEST_DOCUMENT_TYPE_NAME,
            "submittedCharterId",
        ) {
            PropertyReference::Value(DocumentPropertyReferenceTarget::PermanentDocument {
                document_type_name,
                property_agreement,
                ..
            }) => {
                assert_eq!(document_type_name, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME);
                assert_eq!(
                    property_agreement.get("recipientId").map(String::as_str),
                    Some("$ownerId"),
                    "the recipient is the proposal's owner"
                );
            }
            other => panic!("submittedCharterId: {other:?}"),
        }
        match reference(&contract, JOIN_REQUEST_DOCUMENT_TYPE_NAME, "recipientId") {
            PropertyReference::Value(DocumentPropertyReferenceTarget::IdentityPublicKey {
                key_id_property,
                key_requirements,
            }) => {
                assert_eq!(key_id_property, "recipientKeyId");
                assert_eq!(key_requirements.purpose, Some(Purpose::DECRYPTION));
                assert_eq!(
                    key_requirements.bound_to.as_deref(),
                    Some(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME)
                );
            }
            other => panic!("recipientId: {other:?}"),
        }
        match reference(&contract, JOIN_REQUEST_DOCUMENT_TYPE_NAME, "senderKeyId") {
            PropertyReference::KeyId(key_reference) => {
                assert_eq!(
                    key_reference.identity_property,
                    KeyReferenceIdentityProperty::OwnerId
                );
                assert_eq!(
                    key_reference.key_requirements.purpose,
                    Some(Purpose::ENCRYPTION)
                );
                assert_eq!(
                    key_reference.key_requirements.bound_to.as_deref(),
                    Some(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
                );
            }
            other => panic!("senderKeyId: {other:?}"),
        }
        let encrypted_for = document_type(&contract, JOIN_REQUEST_DOCUMENT_TYPE_NAME)
            .flattened_properties()
            .get("encryptedMessage")
            .and_then(|property| property.encrypted_for.clone())
            .expect("the message declares its envelope");
        assert_eq!(
            encrypted_for.recipient,
            EncryptedForRecipient::Property("recipientId".to_string())
        );
        assert_eq!(encrypted_for.recipient_key, "recipientKeyId");
        assert_eq!(encrypted_for.sender_key, "senderKeyId");
        assert_eq!(
            encrypted_for.scheme,
            EncryptionScheme::EcdhSecp256k1Aes256Cbc
        );
    }

    #[test]
    fn should_open_the_contest_only_from_the_leaders_own_proposal() {
        let contract = contract();
        match reference(
            &contract,
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            property_names::SUBMITTED_CHARTER_ID,
        ) {
            PropertyReference::Value(DocumentPropertyReferenceTarget::PermanentDocument {
                document_type_name,
                property_agreement,
                ..
            }) => {
                assert_eq!(document_type_name, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME);
                assert_eq!(
                    property_agreement.get("$ownerId").map(String::as_str),
                    Some("$ownerId")
                );
                assert_eq!(
                    property_agreement
                        .get(property_names::TARGET_CONTRACT_ID)
                        .map(String::as_str),
                    Some(property_names::TARGET_CONTRACT_ID)
                );
            }
            other => panic!("submittedCharterId: {other:?}"),
        }
    }

    #[test]
    fn should_choose_members_only_from_the_proposals_join_requests() {
        let contract = contract();
        let members = document_type(&contract, ELECTED_CHARTER_DOCUMENT_TYPE_NAME)
            .flattened_properties()
            .get(property_names::MEMBERS)
            .expect("members")
            .clone();
        match members.property_type.reference() {
            Some(PropertyReference::Elements {
                target:
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                        document_type_name,
                        lookup,
                        ..
                    },
                max_items,
            }) => {
                assert_eq!(document_type_name, JOIN_REQUEST_DOCUMENT_TYPE_NAME);
                assert_eq!(max_items, 15);
                assert_eq!(lookup.index, "bySubmittedCharter");
                assert_eq!(
                    lookup.keys.get(property_names::SUBMITTED_CHARTER_ID),
                    Some(&LookupKeySource::Property(
                        property_names::SUBMITTED_CHARTER_ID.to_string()
                    ))
                );
                assert_eq!(
                    lookup.keys.get("$ownerId"),
                    Some(&LookupKeySource::ReferenceValue)
                );
            }
            other => panic!("members: {other:?}"),
        }
        let DocumentPropertyType::TypedArray(typed_array) = &members.property_type else {
            panic!("members is a typed array");
        };
        assert!(typed_array.unique_items);
        assert_eq!(typed_array.min_items.unwrap_or_default(), 0);
        assert_eq!(
            members.distinct_from,
            Some(DistinctFrom::OwnerId),
            "the leader cannot list themself"
        );
        let join_request = document_type(&contract, JOIN_REQUEST_DOCUMENT_TYPE_NAME);
        let index = join_request
            .indexes()
            .get("bySubmittedCharter")
            .expect("the join request's unique index");
        assert!(index.unique);
        assert!(
            !join_request.documents_transferable().is_transferable(),
            "a lookup may key on $ownerId only on a type that cannot change hands"
        );
    }

    #[test]
    fn should_round_trip_a_proposal_through_the_system_contract() {
        let contract = contract();
        let proposal = proposal();
        let document = document_with(
            &contract,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            proposal.to_document_properties(),
        );
        assert_eq!(
            schema_validation(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME, &document),
            vec![],
            "the encoded proposal passes the schema"
        );
        let read = SubmittedCharter::from_document_properties(document.properties())
            .into_data()
            .expect("the proposal reads");
        assert_eq!(read, proposal);
    }

    /// A proposal's reward split is held to 100 by a `propertyConstraints` rule, which
    /// the contract's document validation applies, so consensus checks it on every create
    /// and a client validating the document sees the same answer.
    #[test]
    fn should_hold_a_proposal_reward_split_to_one_hundred() {
        let contract = contract();
        let submitted_charter = document_type(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME);
        assert_eq!(
            submitted_charter
                .property_constraints()
                .keys()
                .collect::<Vec<_>>(),
            ["rewardSplitIsWhole"]
        );
        let judge = |leader: u8, equal: u8, actions: u8| {
            let mut proposal = proposal();
            proposal.reward_split = ModerationCharterRewardSplit {
                leader,
                equal,
                actions,
            };
            let document = document_with(
                &contract,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                proposal.to_document_properties(),
            );
            schema_validation(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME, &document)
        };
        assert_eq!(judge(10, 40, 50), vec![]);
        assert_eq!(judge(100, 0, 0), vec![]);
        for (leader, equal, actions) in [(10, 40, 40), (40, 40, 40), (0, 0, 0)] {
            let errors = judge(leader, equal, actions);
            assert!(
                matches!(
                    errors.as_slice(),
                    [ConsensusError::BasicError(
                        BasicError::DocumentPropertyConstraintViolatedError(e)
                    )] if e.constraint() == "rewardSplitIsWhole"
                        && e.violation() == PropertyConstraintViolation::NotMet
                ),
                "{leader} + {equal} + {actions}: {errors:?}"
            );
        }
    }

    #[test]
    fn should_round_trip_an_elected_charter_through_the_system_contract() {
        let contract = contract();
        let charter = ElectedCharter {
            target_contract_id: Identifier::from([9u8; 32]),
            submitted_charter_id: Identifier::from([7u8; 32]),
            members: vec![Identifier::from([2u8; 32]), Identifier::from([5u8; 32])],
        };
        let document = document_with(
            &contract,
            ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
            charter.to_document_properties(),
        );
        assert_eq!(
            schema_validation(&contract, ELECTED_CHARTER_DOCUMENT_TYPE_NAME, &document),
            vec![],
            "the encoded elected charter passes the schema"
        );
        let read = ElectedCharter::from_document_properties(document.properties())
            .into_data()
            .expect("the elected charter reads back");
        assert_eq!(read, charter);
    }

    #[test]
    fn should_refuse_through_the_schema_what_it_can_express() {
        let contract = contract();
        let too_many =
            |count: u8| Value::Array((0..count).map(|i| Value::Identifier([i; 32])).collect());
        for (type_name, property, value, expected_keyword) in [
            (
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::DESCRIPTION,
                Value::Text("a".repeat(4097)),
                "maxLength",
            ),
            (
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::DESCRIPTION,
                Value::Text(String::new()),
                "minLength",
            ),
            (
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::REASONS,
                too_many(65),
                "maxItems",
            ),
            (
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::REASONS,
                Value::Array(vec![Value::Identifier([3; 32]), Value::Identifier([3; 32])]),
                "uniqueItems",
            ),
            (
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::MODERATORS_SHARE,
                Value::U8(101),
                "maximum",
            ),
            (
                ELECTED_CHARTER_DOCUMENT_TYPE_NAME,
                property_names::MEMBERS,
                too_many(16),
                "maxItems",
            ),
        ] {
            let properties = if type_name == SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME {
                proposal().to_document_properties()
            } else {
                ElectedCharter {
                    target_contract_id: Identifier::from([9u8; 32]),
                    submitted_charter_id: Identifier::from([7u8; 32]),
                    members: vec![],
                }
                .to_document_properties()
            };
            let mut document = document_with(&contract, type_name, properties);
            document.set(property, value);
            let errors = schema_validation(&contract, type_name, &document);
            let message = format!("{errors:?}");
            assert!(
                !errors.is_empty() && message.contains(expected_keyword),
                "{type_name}.{property}: expected a {expected_keyword} error, got {message}"
            );
        }
        for property in [
            property_names::TARGET_CONTRACT_ID,
            property_names::DESCRIPTION,
            property_names::REASONS,
            property_names::REWARD_SPLIT,
        ] {
            let mut document = document_with(
                &contract,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                proposal().to_document_properties(),
            );
            document.properties_mut().remove(property);
            assert!(
                !schema_validation(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME, &document)
                    .is_empty(),
                "{property} is required"
            );
        }
    }

    /// The description's cap is 4096 bytes, not just 4096 characters: the schema's `maxBytes`,
    /// which document validation checks after the JSON schema, so clients refuse an oversized
    /// description before they broadcast it.
    #[test]
    fn should_refuse_a_description_over_4096_bytes_within_4096_characters() {
        let contract = contract();
        assert_eq!(
            document_type(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME)
                .flattened_properties()
                .get(property_names::DESCRIPTION)
                .expect("the description")
                .property_type,
            DocumentPropertyType::String(StringPropertySizes {
                min_length: Some(1),
                max_length: Some(4096),
                max_bytes: Some(4096),
            })
        );
        let with_description = |description: String| {
            document_with(
                &contract,
                SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
                SubmittedCharter {
                    description,
                    ..proposal()
                }
                .to_document_properties(),
            )
        };

        // At the cap, in one-byte and in two-byte characters
        for description in ["a".repeat(4096), "é".repeat(2048)] {
            let document = with_description(description);
            assert_eq!(
                schema_validation(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME, &document),
                vec![]
            );
        }

        // 2049 characters are within maxLength, but their 4098 bytes are over maxBytes
        let document = with_description("é".repeat(2049));
        assert!(matches!(
            schema_validation(&contract, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME, &document)
                .as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyMaxBytesExceededError(e))]
                if e.property() == property_names::DESCRIPTION
                    && e.byte_length() == 4098
                    && e.max_bytes() == 4096
        ));
    }

    /// After the election the leader adds members from the same join requests and removes
    /// elected members, and a member asks to leave: each change is written once per member,
    /// and only by the one entitled to it.
    #[test]
    fn should_let_only_the_leader_change_the_team_and_only_a_member_resign() {
        let contract = contract();
        let charter_agreement =
            |type_name| match reference(&contract, type_name, property_names::ELECTED_CHARTER_ID) {
                PropertyReference::Value(DocumentPropertyReferenceTarget::PermanentDocument {
                    document_type_name,
                    property_agreement,
                    ..
                }) => {
                    assert_eq!(document_type_name, ELECTED_CHARTER_DOCUMENT_TYPE_NAME);
                    property_agreement.clone()
                }
                other => panic!("{type_name}.electedCharterId: {other:?}"),
            };

        let added = charter_agreement(ADDED_MODERATOR_DOCUMENT_TYPE_NAME);
        assert_eq!(added.get("$ownerId").map(String::as_str), Some("$ownerId"));
        assert_eq!(
            added
                .get(property_names::SUBMITTED_CHARTER_ID)
                .map(String::as_str),
            Some(property_names::SUBMITTED_CHARTER_ID)
        );
        let removed = charter_agreement(REMOVED_MODERATOR_DOCUMENT_TYPE_NAME);
        assert_eq!(
            removed.get("$ownerId").map(String::as_str),
            Some("$ownerId")
        );
        let resignation = charter_agreement(RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME);
        assert_eq!(
            resignation.get("recipientId").map(String::as_str),
            Some("$ownerId"),
            "a resignation is addressed to the charter's leader"
        );

        match reference(
            &contract,
            ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
            property_names::MEMBER_ID,
        ) {
            PropertyReference::Value(
                DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                    document_type_name,
                    lookup,
                    ..
                },
            ) => {
                assert_eq!(document_type_name, JOIN_REQUEST_DOCUMENT_TYPE_NAME);
                assert_eq!(lookup.index, "bySubmittedCharter");
                assert_eq!(
                    lookup.keys.get("$ownerId"),
                    Some(&LookupKeySource::ReferenceValue)
                );
            }
            other => panic!("addedModerator.memberId: {other:?}"),
        }
        // Only an elected member can be removed; an added one is taken off by deleting the
        // addition
        match reference(
            &contract,
            REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
            property_names::MEMBER_ID,
        ) {
            PropertyReference::Value(DocumentPropertyReferenceTarget::ListElement(listed)) => {
                assert_eq!(
                    listed.document_type_name,
                    ELECTED_CHARTER_DOCUMENT_TYPE_NAME
                );
                assert_eq!(listed.in_list, property_names::MEMBERS);
                assert_eq!(
                    listed.document_id_property(),
                    Some(property_names::ELECTED_CHARTER_ID)
                );
            }
            other => panic!("removedModerator.memberId: {other:?}"),
        }
        for type_name in [
            ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
            REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
        ] {
            let member = document_type(&contract, type_name)
                .flattened_properties()
                .get(property_names::MEMBER_ID)
                .expect("memberId");
            assert_eq!(
                member.distinct_from,
                Some(DistinctFrom::OwnerId),
                "{type_name}: the leader is not a member"
            );
        }

        for (type_name, index_name, member_property) in [
            (
                ADDED_MODERATOR_DOCUMENT_TYPE_NAME,
                "byElectedCharterMember",
                property_names::MEMBER_ID,
            ),
            (
                REMOVED_MODERATOR_DOCUMENT_TYPE_NAME,
                "byElectedCharterMember",
                property_names::MEMBER_ID,
            ),
            (
                RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
                "byElectedCharterOwner",
                "$ownerId",
            ),
        ] {
            let index = document_type(&contract, type_name)
                .indexes()
                .get(index_name)
                .unwrap_or_else(|| panic!("{type_name}.{index_name}"));
            assert!(index.unique, "{type_name}: once per member and charter");
            assert_eq!(
                index
                    .properties
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>(),
                vec![property_names::ELECTED_CHARTER_ID, member_property]
            );
        }
    }

    /// Only a member of the seated team may ask to leave: the writer is listed in the elected
    /// charter's `members`, or the leader added it after the election and has not deleted the
    /// addition. The request is
    /// deletable, which withdraws it, and carries a message only the leader can read.
    #[test]
    fn should_let_only_a_team_member_ask_to_leave() {
        let contract = contract();
        let resignation = document_type(&contract, RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME);
        assert!(resignation.documents_can_be_deleted());

        let Some(DocumentPropertyReferenceTarget::AnyOf(operands)) = resignation.owner_reference()
        else {
            panic!(
                "the writer must meet any of the membership targets: {:?}",
                resignation.owner_reference()
            );
        };
        match operands.operands() {
            [DocumentPropertyReferenceTarget::ListElement(listed), DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                document_type_name,
                lookup,
                ..
            }] => {
                assert_eq!(
                    listed.document_type_name,
                    ELECTED_CHARTER_DOCUMENT_TYPE_NAME
                );
                assert_eq!(listed.in_list, property_names::MEMBERS);
                assert_eq!(
                    listed.document_id_property(),
                    Some(property_names::ELECTED_CHARTER_ID)
                );
                assert_eq!(document_type_name, ADDED_MODERATOR_DOCUMENT_TYPE_NAME);
                assert_eq!(lookup.index, "byElectedCharterMember");
                assert_eq!(
                    lookup.keys.get(property_names::MEMBER_ID),
                    Some(&LookupKeySource::ReferenceValue)
                );
            }
            other => panic!("resignation membership operands: {other:?}"),
        }

        let encrypted_for = resignation
            .flattened_properties()
            .get("encryptedMessage")
            .and_then(|property| property.encrypted_for.clone())
            .expect("the message declares its envelope");
        assert_eq!(
            encrypted_for.recipient,
            EncryptedForRecipient::Property("recipientId".to_string())
        );
        match reference(
            &contract,
            RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
            "senderKeyId",
        ) {
            PropertyReference::KeyId(key_reference) => {
                assert_eq!(
                    key_reference.key_requirements.bound_to.as_deref(),
                    Some(JOIN_REQUEST_DOCUMENT_TYPE_NAME),
                    "the member's encryption key is the one its join request used"
                );
            }
            other => panic!("senderKeyId: {other:?}"),
        }
    }
}
