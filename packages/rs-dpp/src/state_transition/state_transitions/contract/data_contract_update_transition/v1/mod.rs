mod identity_signed;
mod overlapping_entry;
mod registration_cost;
mod state_transition_like;
mod types;
pub(super) mod v1_methods;
mod version;

use std::collections::BTreeMap;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::{BinaryData, Identifier, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::group::Group;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
#[cfg(feature = "serde-conversion")]
use crate::data_contract::serialized_version::{
    deserialize_u16_group_map, deserialize_u16_token_configuration_map,
};
use crate::data_contract::update_values::{DataContractUpdateValues, DescriptionUpdate};
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::identity::KeyID;
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{NonConsensusError, ProtocolError};

/// A delta-based data contract update.
///
/// Where the V0 transition re-sends the whole contract, this one carries
/// only what changed: new and updated document schemas and shared
/// definitions, new groups and tokens, added and removed keywords, and an
/// optional description or config change. Validation fetches the stored
/// contract, merges the delta onto it with
/// [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update),
/// and then validates the result exactly like a V0 update.
///
/// The contract id and owner are explicit fields because nothing else in
/// the transition carries them; the owner must match the stored contract's
/// owner.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, DecodeUntrusted, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractUpdateTransitionV1 {
    #[cfg_attr(
        feature = "serde-conversion",
        serde(rename = "$identity-contract-nonce")
    )]
    pub identity_contract_nonce: IdentityNonce,
    /// The contract being updated.
    pub data_contract_id: Identifier,
    /// The identity submitting the update; it must own the contract.
    pub owner_id: Identifier,
    /// The contract version this update produces: the stored version plus one.
    pub version: u32,
    /// A replacement config, or `None` to keep the stored one.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub config: Option<DataContractConfig>,
    /// New schemas for existing shared `$defs` definitions.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub updated_schema_defs: BTreeMap<DefinitionName, Value>,
    /// Shared `$defs` definitions to add.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub new_schema_defs: BTreeMap<DefinitionName, Value>,
    /// New schemas for existing document types.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub updated_document_schemas: BTreeMap<DocumentName, Value>,
    /// Document types to add.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub new_document_schemas: BTreeMap<DocumentName, Value>,
    /// Groups to add; positions continue the stored ones.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, deserialize_with = "deserialize_u16_group_map")
    )]
    pub new_groups: BTreeMap<GroupContractPosition, Group>,
    /// Tokens to add; positions continue the stored ones.
    #[cfg_attr(
        feature = "serde-conversion",
        serde(default, deserialize_with = "deserialize_u16_token_configuration_map")
    )]
    pub new_tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,
    /// Keywords to add.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub add_keywords: Vec<String>,
    /// Keywords to remove.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub remove_keywords: Vec<String>,
    /// What to do with the description.
    #[cfg_attr(feature = "serde-conversion", serde(default))]
    pub description: DescriptionUpdate,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl From<DataContractUpdateTransitionV1> for StateTransition {
    fn from(value: DataContractUpdateTransitionV1) -> Self {
        let transition: DataContractUpdateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractUpdateTransitionV1> for StateTransition {
    fn from(value: &DataContractUpdateTransitionV1) -> Self {
        let transition: DataContractUpdateTransition = value.clone().into();
        transition.into()
    }
}

impl<'a> From<&'a DataContractUpdateTransitionV1> for DataContractUpdateValues<'a> {
    fn from(transition: &'a DataContractUpdateTransitionV1) -> Self {
        DataContractUpdateValues {
            owner_id: transition.owner_id,
            version: transition.version,
            config: transition.config.as_ref(),
            updated_schema_defs: &transition.updated_schema_defs,
            new_schema_defs: &transition.new_schema_defs,
            updated_document_schemas: &transition.updated_document_schemas,
            new_document_schemas: &transition.new_document_schemas,
            new_groups: &transition.new_groups,
            new_tokens: &transition.new_tokens,
            add_keywords: &transition.add_keywords,
            remove_keywords: &transition.remove_keywords,
            description: &transition.description,
        }
    }
}

fn not_expressible(what: String) -> ProtocolError {
    ProtocolError::NonConsensusError(NonConsensusError::StateTransitionCreationError(format!(
        "a delta-based contract update can not express this change: {what}"
    )))
}

impl DataContractUpdateTransitionV1 {
    /// Computes the delta that turns `old_contract` into `new_contract`.
    ///
    /// The result is unsigned, with a zero key id and fee increase. Changes
    /// the delta cannot express (removing a document type, schema
    /// definition, group or token, changing an existing group or token, or
    /// a different contract id) are rejected: a full-contract V0 transition
    /// would not be accepted for them either, so a caller asking for one
    /// has a bug worth surfacing at construction time.
    pub fn from_contract_update(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<Self, ProtocolError> {
        if old_contract.id() != new_contract.id() {
            return Err(not_expressible(format!(
                "contract id {} differs from {}",
                new_contract.id(),
                old_contract.id()
            )));
        }

        let old_document_schemas = old_contract.document_schemas();
        let new_document_schemas_all = new_contract.document_schemas();
        let mut updated_document_schemas = BTreeMap::new();
        let mut new_document_schemas = BTreeMap::new();
        for (name, schema) in &new_document_schemas_all {
            match old_document_schemas.get(name) {
                Some(old_schema) if *old_schema == *schema => {}
                Some(_) => {
                    updated_document_schemas.insert(name.clone(), (*schema).clone());
                }
                None => {
                    new_document_schemas.insert(name.clone(), (*schema).clone());
                }
            }
        }
        if let Some(removed) = old_document_schemas
            .keys()
            .find(|name| !new_document_schemas_all.contains_key(*name))
        {
            return Err(not_expressible(format!(
                "document type '{removed}' was removed"
            )));
        }

        let empty_definitions = BTreeMap::new();
        let old_schema_defs = old_contract.schema_defs().unwrap_or(&empty_definitions);
        let new_schema_defs_all = new_contract.schema_defs().unwrap_or(&empty_definitions);
        let mut updated_schema_defs = BTreeMap::new();
        let mut new_schema_defs = BTreeMap::new();
        for (name, definition) in new_schema_defs_all {
            match old_schema_defs.get(name) {
                Some(old_definition) if old_definition == definition => {}
                Some(_) => {
                    updated_schema_defs.insert(name.clone(), definition.clone());
                }
                None => {
                    new_schema_defs.insert(name.clone(), definition.clone());
                }
            }
        }
        if let Some(removed) = old_schema_defs
            .keys()
            .find(|name| !new_schema_defs_all.contains_key(*name))
        {
            return Err(not_expressible(format!(
                "schema definition '{removed}' was removed"
            )));
        }

        let mut new_groups = BTreeMap::new();
        for (position, group) in new_contract.groups() {
            match old_contract.groups().get(position) {
                Some(old_group) if old_group == group => {}
                Some(_) => {
                    return Err(not_expressible(format!(
                        "group at position {position} was changed"
                    )))
                }
                None => {
                    new_groups.insert(*position, group.clone());
                }
            }
        }
        if let Some(removed) = old_contract
            .groups()
            .keys()
            .find(|position| !new_contract.groups().contains_key(*position))
        {
            return Err(not_expressible(format!(
                "group at position {removed} was removed"
            )));
        }

        let mut new_tokens = BTreeMap::new();
        for (position, token) in new_contract.tokens() {
            match old_contract.tokens().get(position) {
                Some(old_token) if old_token == token => {}
                Some(_) => {
                    return Err(not_expressible(format!(
                        "token at position {position} was changed"
                    )))
                }
                None => {
                    new_tokens.insert(*position, token.clone());
                }
            }
        }
        if let Some(removed) = old_contract
            .tokens()
            .keys()
            .find(|position| !new_contract.tokens().contains_key(*position))
        {
            return Err(not_expressible(format!(
                "token at position {removed} was removed"
            )));
        }

        let remove_keywords: Vec<String> = old_contract
            .keywords()
            .iter()
            .filter(|keyword| !new_contract.keywords().contains(*keyword))
            .cloned()
            .collect();
        let add_keywords: Vec<String> = new_contract
            .keywords()
            .iter()
            .filter(|keyword| !old_contract.keywords().contains(*keyword))
            .cloned()
            .collect();
        // Applying the delta keeps the stored order of the keywords it
        // leaves in place and appends the added ones, so any other order
        // in the new contract would be silently lost.
        let mut applied_keywords: Vec<String> = old_contract
            .keywords()
            .iter()
            .filter(|keyword| !remove_keywords.contains(*keyword))
            .cloned()
            .collect();
        applied_keywords.extend(add_keywords.iter().cloned());
        if applied_keywords != *new_contract.keywords() {
            return Err(not_expressible(
                "keywords were reordered; a delta keeps the stored order of the keywords it \
                 leaves in place and appends the ones it adds"
                    .to_string(),
            ));
        }

        let description = match (old_contract.description(), new_contract.description()) {
            (old, new) if old == new => DescriptionUpdate::Keep,
            (_, None) => DescriptionUpdate::Clear,
            (_, Some(new)) => DescriptionUpdate::Set(new.clone()),
        };

        let config =
            (old_contract.config() != new_contract.config()).then(|| *new_contract.config());

        Ok(DataContractUpdateTransitionV1 {
            identity_contract_nonce,
            data_contract_id: new_contract.id(),
            owner_id: new_contract.owner_id(),
            version: new_contract.version(),
            config,
            updated_schema_defs,
            new_schema_defs,
            updated_document_schemas,
            new_document_schemas,
            new_groups,
            new_tokens,
            add_keywords,
            remove_keywords,
            description,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

#[cfg(all(test, feature = "validation"))]
mod tests {
    use super::*;
    use crate::block::block_info::BlockInfo;
    use crate::block::epoch::Epoch;
    use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Setters;
    use crate::data_contract::accessors::v1::DataContractV1Setters;
    use crate::data_contract::group::v0::GroupV0;
    use crate::data_contract::serialized_version::{
        DataContractInSerializationFormat, DataContractMismatch,
    };
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;

    fn contracts() -> (DataContract, DataContract, &'static PlatformVersion) {
        let platform_version = PlatformVersion::latest();
        let old_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let new_contract = old_contract.clone();
        (old_contract, new_contract, platform_version)
    }

    fn block_info() -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000,
            height: 42,
            core_height: 7,
            epoch: Epoch::new(3).expect("epoch"),
        }
    }

    fn new_document_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0
                }
            },
            "additionalProperties": false
        })
    }

    fn apply(old_contract: &DataContract, delta: &DataContractUpdateTransitionV1) -> DataContract {
        let result = old_contract
            .apply_update(
                delta.into(),
                &block_info(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("apply_update should not fail with a protocol error");
        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        result.into_data().expect("updated contract")
    }

    /// The contract the delta produces, as the merge alone builds it.
    fn merged(
        old_contract: &DataContract,
        delta: &DataContractUpdateTransitionV1,
    ) -> DataContractInSerializationFormat {
        DataContractInSerializationFormat::V1(
            DataContractUpdateValues::from(delta)
                .merge_onto(old_contract, &block_info())
                .expect("the delta applies to the stored contract"),
        )
    }

    fn format(contract: &DataContract) -> DataContractInSerializationFormat {
        contract
            .clone()
            .try_into_platform_versioned(PlatformVersion::latest())
            .expect("serialization format")
    }

    #[test]
    fn delta_of_identical_contracts_is_empty() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 5)
                .expect("delta");

        assert_eq!(delta.identity_contract_nonce, 5);
        assert_eq!(delta.data_contract_id, old_contract.id());
        assert_eq!(delta.owner_id, old_contract.owner_id());
        assert_eq!(delta.version, new_contract.version());
        assert!(delta.config.is_none());
        assert!(delta.updated_schema_defs.is_empty());
        assert!(delta.new_schema_defs.is_empty());
        assert!(delta.updated_document_schemas.is_empty());
        assert!(delta.new_document_schemas.is_empty());
        assert!(delta.new_groups.is_empty());
        assert!(delta.new_tokens.is_empty());
        assert!(delta.add_keywords.is_empty());
        assert!(delta.remove_keywords.is_empty());
        assert_eq!(delta.description, DescriptionUpdate::Keep);
    }

    #[test]
    fn applying_the_delta_reproduces_the_new_contract() {
        let (old_contract, mut new_contract, platform_version) = contracts();
        new_contract.increment_version();
        new_contract
            .set_document_schema(
                "newType",
                new_document_schema(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("new document type");
        new_contract.add_group(
            0,
            Group::V0(GroupV0 {
                members: [(old_contract.owner_id(), 1), (Identifier::random(), 1)]
                    .into_iter()
                    .collect(),
                required_power: 2,
            }),
        );
        new_contract.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);
        new_contract.set_description(Some("a described contract".to_string()));

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");

        assert_eq!(delta.new_document_schemas.len(), 1);
        assert_eq!(delta.new_groups.len(), 1);
        assert_eq!(delta.add_keywords, vec!["alpha", "beta"]);
        assert_eq!(
            delta.description,
            DescriptionUpdate::Set("a described contract".to_string())
        );

        let updated_contract = apply(&old_contract, &delta);

        assert_eq!(updated_contract.version(), new_contract.version());
        assert_eq!(
            updated_contract.document_schemas(),
            new_contract.document_schemas()
        );
        assert_eq!(updated_contract.groups(), new_contract.groups());
        assert_eq!(updated_contract.keywords(), new_contract.keywords());
        assert_eq!(updated_contract.description(), new_contract.description());
        assert_eq!(updated_contract.created_at(), old_contract.created_at());
        assert_eq!(updated_contract.updated_at(), Some(block_info().time_ms));
        assert_eq!(updated_contract.updated_at_block_height(), Some(42));
        assert_eq!(updated_contract.updated_at_epoch(), Some(3));
        // the merge alone builds the same contract apply_update validated
        assert_eq!(
            format(&updated_contract).first_mismatch(&merged(&old_contract, &delta)),
            None
        );
    }

    #[test]
    fn removing_and_clearing_are_expressed_and_applied() {
        let (mut old_contract, _, platform_version) = contracts();
        old_contract.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);
        old_contract.set_description(Some("old description".to_string()));
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_keywords(vec!["beta".to_string(), "gamma".to_string()]);
        new_contract.set_description(None);
        let _ = platform_version;

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");

        assert_eq!(delta.remove_keywords, vec!["alpha"]);
        assert_eq!(delta.add_keywords, vec!["gamma"]);
        assert_eq!(delta.description, DescriptionUpdate::Clear);

        let updated_contract = apply(&old_contract, &delta);
        assert_eq!(
            updated_contract.keywords(),
            &vec!["beta".to_string(), "gamma".to_string()]
        );
        assert_eq!(updated_contract.description(), None);
        assert_eq!(
            format(&updated_contract).first_mismatch(&merged(&old_contract, &delta)),
            None
        );
        // the stored contract itself is not what the delta produces
        assert_eq!(
            format(&old_contract).first_mismatch(&merged(&old_contract, &delta)),
            Some(DataContractMismatch::Version)
        );
    }

    #[test]
    fn a_removed_document_type_can_not_be_expressed() {
        let (mut old_contract, new_contract, platform_version) = contracts();
        old_contract
            .set_document_schema(
                "extraType",
                new_document_schema(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("extra document type");

        let result =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1);

        assert_matches!(
            result,
            Err(ProtocolError::NonConsensusError(
                NonConsensusError::StateTransitionCreationError(message)
            )) if message.contains("document type 'extraType' was removed")
        );
    }

    #[test]
    fn a_different_owner_is_rejected_when_applied() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let mut delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");
        delta.owner_id = Identifier::random();

        let result = old_contract
            .apply_update(
                (&delta).into(),
                &block_info(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("no protocol error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdatePermissionError(e))]
                if *e.data_contract_id() == old_contract.id() && *e.identity_id() == delta.owner_id
        );
    }

    #[test]
    fn a_new_document_type_that_already_exists_is_rejected_when_applied() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let mut delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");
        delta
            .new_document_schemas
            .insert("niceDocument".to_string(), new_document_schema());

        let result = old_contract
            .apply_update(
                (&delta).into(),
                &block_info(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("no protocol error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryAlreadyExistsError(e))]
                if e.entry_kind() == DataContractUpdateEntryKind::DocumentType && e.name() == "niceDocument"
        );
    }

    #[test]
    fn an_updated_document_type_that_does_not_exist_is_rejected_when_applied() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let mut delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");
        delta
            .updated_document_schemas
            .insert("missingType".to_string(), new_document_schema());

        let result = old_contract
            .apply_update(
                (&delta).into(),
                &block_info(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("no protocol error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryNotFoundError(e))]
                if e.entry_kind() == DataContractUpdateEntryKind::DocumentType && e.name() == "missingType"
        );
    }

    #[test]
    fn a_removed_keyword_that_is_not_present_is_rejected_when_applied() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let mut delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("delta");
        delta.remove_keywords.push("ghost".to_string());

        let result = old_contract
            .apply_update(
                (&delta).into(),
                &block_info(),
                true,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("no protocol error");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdateEntryNotFoundError(e))]
                if e.entry_kind() == DataContractUpdateEntryKind::Keyword && e.name() == "ghost"
        );
    }

    #[test]
    fn reordered_keywords_can_not_be_expressed() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_keywords(vec!["one".to_string(), "two".to_string()]);
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_keywords(vec!["two".to_string(), "one".to_string()]);

        let result =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1);

        assert_matches!(
            result,
            Err(ProtocolError::NonConsensusError(
                NonConsensusError::StateTransitionCreationError(message)
            )) if message.contains("keywords were reordered")
        );
    }

    #[test]
    fn removing_and_appending_keywords_keeps_the_stored_order() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_keywords(vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
        ]);
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_keywords(vec![
            "one".to_string(),
            "three".to_string(),
            "four".to_string(),
        ]);

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1)
                .expect("removing one keyword and appending another is expressible");

        assert_eq!(delta.remove_keywords, vec!["two".to_string()]);
        assert_eq!(delta.add_keywords, vec!["four".to_string()]);
        assert_eq!(
            apply(&old_contract, &delta).keywords(),
            new_contract.keywords()
        );
    }

    #[test]
    fn the_delta_round_trips_through_the_wire_format() {
        use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

        let (old_contract, mut new_contract, platform_version) = contracts();
        new_contract.increment_version();
        new_contract
            .set_document_schema(
                "newType",
                new_document_schema(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("new document type");
        new_contract.set_keywords(vec!["alpha".to_string()]);

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 9)
                .expect("delta");
        let state_transition: StateTransition = delta.clone().into();

        let bytes = state_transition.serialize_to_bytes().expect("serialize");
        let recovered =
            StateTransition::deserialize_from_bytes_untrusted(&bytes).expect("deserialize");

        assert_matches!(
            recovered,
            StateTransition::DataContractUpdate(DataContractUpdateTransition::V1(recovered))
                if recovered == delta
        );
    }
}
