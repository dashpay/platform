use crate::error::Error;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::Network;
use dpp::data_contract::document_type::schema::{MemberPositionOrder, MemberPositionOrderError};
use dpp::data_contract::errors::DataContractError;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use super::v2::DataContractCreateStateTransitionBasicStructureValidationV2;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV3
{
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV3 for DataContractCreateTransition {
    /// Generation 2, once every object of the contract's document schemas and of its `$defs`
    /// lists its members at consecutive positions, and every `$ref` points into its `$defs` at
    /// a schema whose objects do (protocol version 15).
    ///
    /// An object is stored with its members in the order its `properties` map lists them, while
    /// a document type's own properties are stored in `position` order. Requiring a new
    /// contract's objects to list their members in position order makes the two orders agree,
    /// so its objects are stored as its document types are. Contracts registered earlier keep
    /// their order and are not judged by it.
    fn validate_basic_structure_v3(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let v2_result = self.validate_basic_structure_v2(network_type, platform_version)?;
        if !v2_result.is_valid() {
            return Ok(v2_result);
        }

        let data_contract = self.data_contract();
        let error = MemberPositionOrder::new(None, data_contract.schema_defs(), platform_version)
            .first_error_of_registration(data_contract.document_schemas().values());
        if let Some(
            MemberPositionOrderError::ListedOutOfPositionOrder(details)
            | MemberPositionOrderError::AddedBeforeExistingMembers(details),
        ) = error
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(details),
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
    use assert_matches::assert_matches;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::{platform_value, Value};
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::TryIntoPlatformVersioned;
    use std::collections::BTreeMap;

    fn string_member(name: &str, position: u64) -> (Value, Value) {
        (
            Value::Text(name.to_string()),
            platform_value!({ "type": "string", "maxLength": 20, "position": position }),
        )
    }

    fn object(members: Vec<(Value, Value)>, position: u64) -> Value {
        platform_value!({
            "type": "object",
            "position": position,
            "properties": Value::Map(members),
            "additionalProperties": false,
        })
    }

    /// The fixture contract with a `page` type holding the object `metadata`, and the
    /// `$defs` given.
    fn create_transition(
        metadata: Value,
        defs: Option<BTreeMap<String, Value>>,
        platform_version: &PlatformVersion,
    ) -> DataContractCreateTransition {
        let identity_nonce = IdentityNonce::default();
        let data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();
        let mut carried: DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("expected the serialized form");
        carried
            .document_schemas_mut()
            .expect("expected a transport format's schemas")
            .insert(
                "page".to_string(),
                platform_value!({
                    "type": "object",
                    "properties": { "metadata": metadata },
                    "additionalProperties": false,
                }),
            );
        if let Some(defs) = defs {
            match &mut carried {
                DataContractInSerializationFormat::V0(v0) => v0.schema_defs = Some(defs),
                DataContractInSerializationFormat::V1(v1) => v1.schema_defs = Some(defs),
                DataContractInSerializationFormat::V2(_) => {
                    panic!("expected a transport format")
                }
            }
        }
        DataContractCreateTransitionV0 {
            data_contract: carried,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    fn refused_as_out_of_position_order(result: &SimpleConsensusValidationResult, path: &str) {
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(details)
            ))] if details.contains(&format!("'{path}'"))
        );
    }

    /// From protocol version 15 a new contract's objects list their members in position
    /// order, at every depth and in its `$defs`, so they are stored in it.
    #[test]
    fn should_refuse_a_new_contract_whose_object_lists_its_members_out_of_position_order() {
        let platform_version = PlatformVersion::latest();
        let in_order = object(vec![string_member("title", 0), string_member("slug", 1)], 0);
        let result = create_transition(in_order, None, platform_version)
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("expected the structure to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);

        let out_of_order = object(
            vec![
                string_member("slug", 2),
                string_member("title", 0),
                string_member("description", 1),
            ],
            0,
        );
        refused_as_out_of_position_order(
            &create_transition(out_of_order.clone(), None, platform_version)
                .validate_basic_structure(Network::Testnet, platform_version)
                .expect("expected the structure to be judged"),
            "metadata",
        );

        let nested = object(
            vec![
                string_member("title", 0),
                (Value::Text("inner".to_string()), {
                    let mut inner = out_of_order.clone();
                    inner
                        .insert("position".to_string(), Value::U64(1))
                        .expect("expected a map");
                    inner
                }),
            ],
            0,
        );
        refused_as_out_of_position_order(
            &create_transition(nested, None, platform_version)
                .validate_basic_structure(Network::Testnet, platform_version)
                .expect("expected the structure to be judged"),
            "metadata.inner",
        );

        let through_ref = platform_value!({ "$ref": "#/$defs/meta", "position": 0 });
        let mut definition = out_of_order;
        definition.remove("position").expect("expected a map");
        refused_as_out_of_position_order(
            &create_transition(
                through_ref,
                Some(BTreeMap::from([("meta".to_string(), definition)])),
                platform_version,
            )
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("expected the structure to be judged"),
            "#/$defs/meta",
        );
    }

    /// The members of a new contract's objects take consecutive integer positions, and a `$ref`
    /// points into its `$defs`; a pointer to a definition's scalar member is accepted.
    #[test]
    fn should_refuse_a_new_contract_whose_object_skips_repeats_or_misreads_a_position() {
        let platform_version = PlatformVersion::latest();
        let float_member = (
            Value::Text("b".to_string()),
            platform_value!({ "type": "string", "maxLength": 20, "position": 1.0 }),
        );
        for (case, metadata, fragment) in [
            (
                "a gap",
                object(vec![string_member("a", 0), string_member("b", 2)], 0),
                "leaving a gap",
            ),
            (
                "a repeat",
                object(vec![string_member("a", 0), string_member("b", 0)], 0),
                "as 'a' does",
            ),
            (
                "a float",
                object(vec![string_member("a", 0), float_member], 0),
                "needs a position",
            ),
            (
                "a $ref outside $defs",
                platform_value!({ "$ref": "#/properties/other", "position": 0 }),
                "points into its `$defs`",
            ),
        ] {
            let result = create_transition(metadata, None, platform_version)
                .validate_basic_structure(Network::Testnet, platform_version)
                .expect("expected the structure to be judged");
            assert!(
                matches!(
                    result.errors.as_slice(),
                    [ConsensusError::BasicError(BasicError::ContractError(
                        DataContractError::InvalidContractStructure(details)
                    ))] if details.contains(fragment)
                ),
                "{case}: {:?}",
                result.errors
            );
        }

        let keys = BTreeMap::from([(
            "keys".to_string(),
            platform_value!({
                "type": "object",
                "properties": {
                    "recipient": { "type": "string", "maxLength": 20, "position": 0 },
                },
                "additionalProperties": false,
            }),
        )]);
        let result = create_transition(
            platform_value!({ "$ref": "#/$defs/keys/properties/recipient", "position": 0 }),
            Some(keys),
            platform_version,
        )
        .validate_basic_structure(Network::Testnet, platform_version)
        .expect("expected the structure to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    /// Protocol version 14 registers such a contract, as it always did.
    #[test]
    fn should_keep_registering_an_object_out_of_position_order_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let out_of_order = object(vec![string_member("slug", 1), string_member("title", 0)], 0);
        let result = create_transition(out_of_order, None, platform_version)
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("expected the structure to be judged");
        assert!(result.is_valid(), "{:?}", result.errors);
    }
}
