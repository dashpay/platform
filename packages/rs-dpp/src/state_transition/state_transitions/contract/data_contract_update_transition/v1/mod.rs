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
use crate::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use crate::state_transition::StateTransition;
use crate::validation::ConsensusValidationResult;
use crate::{NonConsensusError, ProtocolError};
use platform_version::version::PlatformVersion;

/// A delta contract update: version 1 of the data contract update transition, from protocol
/// version 15.
///
/// Version 0 carries the whole contract. This one carries only what the update changes: new
/// and replaced document schemas and shared definitions, new groups and tokens, added and
/// removed keywords, and optionally a description or a config. A node reads the stored
/// contract, merges the delta onto it with
/// [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update), and holds
/// the result to every rule of an update that carries the whole contract.
///
/// As a transition is capped well below the size a contract may take, version 0 cannot grow a
/// contract past the transition cap. A delta can, a transition at a time, up to
/// `max_serialized_size`.
///
/// The contract id and the owner are fields of their own because nothing else in the
/// transition carries them; the owner must be the stored contract's owner.
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

impl DataContractUpdateTransitionV1 {
    /// The update carrying the whole contract that this delta stands for on `stored_contract`:
    /// the delta merged onto it by [`DataContract::apply_update`], under the delta's nonce,
    /// fee increase and signature. A node validates that update in the delta's place, so a
    /// delta is held to every rule of an update that carries the whole contract. Its signature
    /// is the delta's, which signs the delta and not this update, and is checked on the delta.
    ///
    /// # Returns
    /// - The update, or the consensus error that says why the delta does not fit the stored
    ///   contract.
    /// - `Err(ProtocolError)` when `platform_version` predates delta contract updates.
    pub fn to_full_contract_update(
        &self,
        stored_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<DataContractUpdateTransitionV0>, ProtocolError> {
        Ok(stored_contract
            .apply_update(self.into(), platform_version)?
            .map(|data_contract| DataContractUpdateTransitionV0 {
                identity_contract_nonce: self.identity_contract_nonce,
                data_contract,
                user_fee_increase: self.user_fee_increase,
                signature_public_key_id: self.signature_public_key_id,
                signature: self.signature.clone(),
            }))
    }
}

fn not_expressible(what: String) -> ProtocolError {
    ProtocolError::NonConsensusError(NonConsensusError::StateTransitionCreationError(format!(
        "a delta contract update cannot express this change: {what}"
    )))
}

impl DataContractUpdateTransitionV1 {
    /// Computes the delta that turns `old_contract`, the stored contract, into `new_contract`.
    ///
    /// The result is unsigned, with a zero key id and fee increase. A change a delta cannot
    /// express is refused here: a removed document type, schema definition, group or token, a
    /// changed group or token, another contract id (none of which a contract update may make
    /// in either form), reordered keywords, or an empty `$defs` map that appears or
    /// disappears.
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

        // `$defs` is optional, and an absent map is not the contract an empty one is. The
        // merge creates the map with the first definition it adds and never removes it, so a
        // delta can neither add an empty map nor drop one. A validated contract never holds an
        // empty one, so only an unvalidated input gets here.
        if old_contract.schema_defs().is_none() != new_contract.schema_defs().is_none()
            && new_contract.schema_defs().is_none_or(BTreeMap::is_empty)
        {
            return Err(not_expressible(
                "the presence of the schema definitions map changed without a definition being added"
                    .to_string(),
            ));
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
        // The merge keeps the stored order of the keywords it leaves in place and appends the
        // added ones, so any other order in the new contract would be silently lost
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
            (old_contract.config() != new_contract.config()).then(|| new_contract.config().clone());

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
    use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Setters;
    use crate::data_contract::accessors::v1::DataContractV1Setters;
    use crate::data_contract::group::v0::GroupV0;
    use crate::data_contract::serialized_version::{
        DataContractInSerializationFormat, DataContractMismatch,
    };
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_version::TryIntoPlatformVersioned;

    fn contracts() -> (DataContract, DataContract, &'static PlatformVersion) {
        let platform_version = PlatformVersion::latest();
        let old_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let new_contract = old_contract.clone();
        (old_contract, new_contract, platform_version)
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

    fn delta_between(
        old_contract: &DataContract,
        new_contract: &DataContract,
    ) -> DataContractUpdateTransitionV1 {
        DataContractUpdateTransitionV1::from_contract_update(old_contract, new_contract, 1)
            .expect("expected the update to be expressible as a delta")
    }

    fn merge(
        old_contract: &DataContract,
        delta: &DataContractUpdateTransitionV1,
    ) -> ConsensusValidationResult<DataContractInSerializationFormat> {
        old_contract
            .apply_update(delta.into(), PlatformVersion::latest())
            .expect("expected the merge to be active at the latest protocol version")
    }

    fn merged(
        old_contract: &DataContract,
        delta: &DataContractUpdateTransitionV1,
    ) -> DataContractInSerializationFormat {
        let result = merge(old_contract, delta);
        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
        result.into_data().expect("expected the merged contract")
    }

    fn format(contract: &DataContract) -> DataContractInSerializationFormat {
        contract
            .clone()
            .try_into_platform_versioned(PlatformVersion::latest())
            .expect("expected the contract in the format transitions carry")
    }

    /// The fixture contract with one document type that references no definition and the
    /// given `$defs`, built without validation, which a contract holding an empty `$defs` map
    /// does not pass.
    fn contract_with_schema_defs(
        base: &DataContract,
        schema_defs: Option<BTreeMap<DefinitionName, Value>>,
    ) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let DataContractInSerializationFormat::V1(mut v1) = format(base) else {
            panic!("expected the latest protocol version to carry contracts in format version 1");
        };
        v1.document_schemas = BTreeMap::from([("note".to_string(), new_document_schema())]);
        v1.schema_defs = schema_defs;
        DataContract::try_from_platform_versioned(
            DataContractInSerializationFormat::V1(v1),
            false,
            &mut vec![],
            platform_version,
        )
        .expect("expected an unvalidated contract with the given schema definitions")
    }

    fn assert_not_expressible(
        result: Result<DataContractUpdateTransitionV1, ProtocolError>,
        reason: &str,
    ) {
        assert_matches!(
            result,
            Err(ProtocolError::NonConsensusError(
                NonConsensusError::StateTransitionCreationError(message)
            )) if message.contains(reason)
        );
    }

    #[test]
    fn should_build_an_empty_delta_for_a_version_bump() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();

        let delta =
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 5)
                .expect("expected a delta");

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
    fn should_merge_into_the_contract_the_delta_was_built_from() {
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
            .expect("expected to add a document type");
        let mut replaced_schema = old_contract
            .document_schemas()
            .get("niceDocument")
            .cloned()
            .cloned()
            .expect("expected the fixture's niceDocument type");
        replaced_schema
            .set_value("description", Value::Text("a nicer document".to_string()))
            .expect("expected to describe the type");
        new_contract
            .set_document_schema(
                "niceDocument",
                replaced_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to replace a document type");
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

        let delta = delta_between(&old_contract, &new_contract);

        assert_eq!(
            delta.new_document_schemas.keys().collect::<Vec<_>>(),
            ["newType"]
        );
        assert_eq!(
            delta.updated_document_schemas.keys().collect::<Vec<_>>(),
            ["niceDocument"]
        );
        assert_eq!(delta.new_groups.len(), 1);
        assert_eq!(delta.add_keywords, ["alpha", "beta"]);
        assert_eq!(
            delta.description,
            DescriptionUpdate::Set("a described contract".to_string())
        );
        assert_eq!(
            merged(&old_contract, &delta).first_mismatch(&format(&new_contract)),
            None
        );
    }

    /// The merge leaves the time fields to its caller: a block stamps the update's, and the
    /// verification of an execution proof does not compare them.
    #[test]
    fn should_keep_the_stored_time_fields_when_merged() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_created_at(Some(1_700_000_000_000));
        old_contract.set_updated_at(Some(1_700_000_100_000));
        old_contract.set_created_at_block_height(Some(40));
        old_contract.set_updated_at_block_height(Some(41));
        old_contract.set_created_at_epoch(Some(2));
        old_contract.set_updated_at_epoch(Some(3));
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();

        let DataContractInSerializationFormat::V1(merged) =
            merged(&old_contract, &delta_between(&old_contract, &new_contract))
        else {
            panic!("expected generation 0 of the merge to build format version 1");
        };

        assert_eq!(merged.version, new_contract.version());
        assert_eq!(merged.created_at, Some(1_700_000_000_000));
        assert_eq!(merged.updated_at, Some(1_700_000_100_000));
        assert_eq!(merged.created_at_block_height, Some(40));
        assert_eq!(merged.updated_at_block_height, Some(41));
        assert_eq!(merged.created_at_epoch, Some(2));
        assert_eq!(merged.updated_at_epoch, Some(3));
    }

    #[test]
    fn should_express_removed_keywords_and_a_cleared_description() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);
        old_contract.set_description(Some("old description".to_string()));
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_keywords(vec!["beta".to_string(), "gamma".to_string()]);
        new_contract.set_description(None);

        let delta = delta_between(&old_contract, &new_contract);

        assert_eq!(delta.remove_keywords, ["alpha"]);
        assert_eq!(delta.add_keywords, ["gamma"]);
        assert_eq!(delta.description, DescriptionUpdate::Clear);
        let merged = merged(&old_contract, &delta);
        assert_eq!(merged.first_mismatch(&format(&new_contract)), None);
        // the stored contract is not what the delta asks for
        assert_eq!(
            merged.first_mismatch(&format(&old_contract)),
            Some(DataContractMismatch::Version)
        );
    }

    #[test]
    fn should_keep_the_stored_keyword_order_and_append_the_added_ones() {
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

        let delta = delta_between(&old_contract, &new_contract);

        assert_eq!(delta.remove_keywords, ["two"]);
        assert_eq!(delta.add_keywords, ["four"]);
        assert_eq!(
            merged(&old_contract, &delta).keywords(),
            new_contract.keywords()
        );
    }

    #[test]
    fn should_refuse_to_build_a_delta_that_reorders_keywords() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_keywords(vec!["one".to_string(), "two".to_string()]);
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_keywords(vec!["two".to_string(), "one".to_string()]);

        assert_not_expressible(
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1),
            "keywords were reordered",
        );
    }

    #[test]
    fn should_refuse_to_build_a_delta_that_removes_a_document_type() {
        let (mut old_contract, new_contract, platform_version) = contracts();
        old_contract
            .set_document_schema(
                "extraType",
                new_document_schema(),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add a document type");

        assert_not_expressible(
            DataContractUpdateTransitionV1::from_contract_update(&old_contract, &new_contract, 1),
            "document type 'extraType' was removed",
        );
    }

    #[test]
    fn should_express_the_first_schema_definition_of_a_contract() {
        let (base, _, _) = contracts();
        let old_contract = contract_with_schema_defs(&base, None);
        let definitions =
            BTreeMap::from([("label".to_string(), platform_value!({"type": "string"}))]);
        let mut new_contract = contract_with_schema_defs(&base, Some(definitions.clone()));
        new_contract.increment_version();

        let delta = delta_between(&old_contract, &new_contract);

        assert_eq!(delta.new_schema_defs, definitions);
        assert!(delta.updated_schema_defs.is_empty());
        assert_eq!(
            merged(&old_contract, &delta).first_mismatch(&format(&new_contract)),
            None
        );
    }

    /// A delta creates `$defs` with the first definition it adds and never removes it, so an
    /// empty map can neither appear nor disappear. Without the refusal the delta would be
    /// empty and merge into another contract than the one asked for.
    #[test]
    fn should_refuse_to_build_a_delta_that_adds_or_drops_an_empty_schema_definitions_map() {
        let (base, _, _) = contracts();
        for (old_definitions, new_definitions) in
            [(None, Some(BTreeMap::new())), (Some(BTreeMap::new()), None)]
        {
            let old_contract = contract_with_schema_defs(&base, old_definitions);
            let mut new_contract = contract_with_schema_defs(&base, new_definitions);
            new_contract.increment_version();

            assert_not_expressible(
                DataContractUpdateTransitionV1::from_contract_update(
                    &old_contract,
                    &new_contract,
                    1,
                ),
                "presence of the schema definitions map",
            );
        }
    }

    #[test]
    fn should_not_merge_a_delta_of_an_identity_that_does_not_own_the_contract() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let mut delta = delta_between(&old_contract, &new_contract);
        delta.owner_id = Identifier::random();

        let result = merge(&old_contract, &delta);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DataContractUpdatePermissionError(e))]
                if *e.data_contract_id() == old_contract.id() && *e.identity_id() == delta.owner_id
        );
    }

    #[test]
    fn should_not_merge_a_new_entry_the_contract_already_has() {
        let (mut old_contract, _, _) = contracts();
        old_contract.set_keywords(vec!["alpha".to_string()]);
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        let delta = delta_between(&old_contract, &new_contract);

        let mut with_document_type = delta.clone();
        with_document_type
            .new_document_schemas
            .insert("niceDocument".to_string(), new_document_schema());
        let mut with_keyword = delta;
        with_keyword.add_keywords.push("alpha".to_string());

        for (delta, kind, name) in [
            (
                with_document_type,
                DataContractUpdateEntryKind::DocumentType,
                "niceDocument",
            ),
            (with_keyword, DataContractUpdateEntryKind::Keyword, "alpha"),
        ] {
            assert_matches!(
                merge(&old_contract, &delta).errors.as_slice(),
                [ConsensusError::StateError(StateError::DataContractUpdateEntryAlreadyExistsError(e))]
                    if e.entry_kind() == kind && e.name() == name
            );
        }
    }

    #[test]
    fn should_not_merge_a_replaced_or_removed_entry_the_contract_does_not_have() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let delta = delta_between(&old_contract, &new_contract);

        let mut with_document_type = delta.clone();
        with_document_type
            .updated_document_schemas
            .insert("missingType".to_string(), new_document_schema());
        let mut with_definition = delta.clone();
        with_definition.updated_schema_defs.insert(
            "missingDefinition".to_string(),
            platform_value!({"type": "string"}),
        );
        let mut with_keyword = delta;
        with_keyword.remove_keywords.push("ghost".to_string());

        for (delta, kind, name) in [
            (
                with_document_type,
                DataContractUpdateEntryKind::DocumentType,
                "missingType",
            ),
            (
                with_definition,
                DataContractUpdateEntryKind::SchemaDef,
                "missingDefinition",
            ),
            (with_keyword, DataContractUpdateEntryKind::Keyword, "ghost"),
        ] {
            assert_matches!(
                merge(&old_contract, &delta).errors.as_slice(),
                [ConsensusError::StateError(StateError::DataContractUpdateEntryNotFoundError(e))]
                    if e.entry_kind() == kind && e.name() == name
            );
        }
    }

    #[test]
    fn should_not_merge_before_protocol_version_15() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let delta = delta_between(&old_contract, &new_contract);
        let platform_version_14 =
            PlatformVersion::get(14).expect("expected protocol version 14 to exist");

        assert_matches!(
            old_contract.apply_update((&delta).into(), platform_version_14),
            Err(ProtocolError::UnknownVersionError(message)) if message.contains("not active")
        );
    }

    #[test]
    fn should_round_trip_through_the_wire_format() {
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
            .expect("expected to add a document type");
        new_contract.set_keywords(vec!["alpha".to_string()]);
        new_contract.set_description(Some("a described contract".to_string()));

        let delta = delta_between(&old_contract, &new_contract);
        let state_transition: StateTransition = delta.clone().into();

        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize the delta");
        let recovered = StateTransition::deserialize_from_bytes_untrusted(&bytes)
            .expect("expected to deserialize the delta");

        assert_matches!(
            recovered,
            StateTransition::DataContractUpdate(DataContractUpdateTransition::V1(recovered))
                if recovered == delta
        );
    }

    /// A delta is refused at decode before protocol version 15, without a fee, as software
    /// that predates it refuses it.
    #[test]
    fn should_be_active_from_protocol_version_15_only() {
        let (old_contract, mut new_contract, _) = contracts();
        new_contract.increment_version();
        let state_transition: StateTransition = delta_between(&old_contract, &new_contract).into();
        let bytes = state_transition
            .serialize_to_bytes()
            .expect("expected to serialize the delta");

        assert_matches!(
            StateTransition::deserialize_from_bytes_untrusted_in_version(
                &bytes,
                PlatformVersion::get(14).expect("expected protocol version 14 to exist"),
            ),
            Err(ProtocolError::StateTransitionError(_))
        );
        assert_matches!(
            StateTransition::deserialize_from_bytes_untrusted_in_version(
                &bytes,
                PlatformVersion::get(15).expect("expected protocol version 15 to exist"),
            ),
            Ok(StateTransition::DataContractUpdate(
                DataContractUpdateTransition::V1(_)
            ))
        );
    }
}
