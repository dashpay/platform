use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Getters;
use crate::data_contract::document_type::property_names::INDICES;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::fee::Credits;
use platform_value::Value;
use platform_version::version::PlatformVersion;

/// The key that makes an index contested.
const CONTESTED: &str = "contested";
/// The key that makes an index unique.
const UNIQUE: &str = "unique";

impl DataContractInSerializationFormat {
    /// What [`registration_cost_v2`](Self::registration_cost_v2) computes, but an index is
    /// billed by the keys it declares instead of by parsing it: contested when it has a
    /// `contested`, otherwise unique when its last boolean `unique` is `true`, as the index
    /// parser reads the two.
    ///
    /// Parsing an index compiles every `regexPattern` of its contested `fieldMatches`. The
    /// balance pre-check runs this before contract registration and update hold the patterns
    /// to `max_contract_schema_patterns` and `max_schema_pattern_compiled_size`, so a parse
    /// here compiled any number of them, each within the `regex` crate's 10 MiB default.
    /// Every index of a contract that registers parses, so such a contract costs here what it
    /// costs under generation 2. An index that does not parse is billed as well, and the
    /// contract is refused once it is parsed.
    pub(super) fn registration_cost_v3(&self, platform_version: &PlatformVersion) -> Credits {
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut cost = fee_version.base_contract_registration_fee;

        for document_type_schema in self.document_schemas().values() {
            cost = cost.saturating_add(fee_version.document_type_registration_fee);

            let Ok(schema_map) = document_type_schema.to_map() else {
                continue;
            };
            let Ok(Some(index_values)) =
                Value::inner_optional_array_slice_value(schema_map, INDICES)
            else {
                continue;
            };
            for index_value in index_values {
                let Ok(index_value_map) = index_value.to_map() else {
                    continue;
                };
                let contested = index_value_map
                    .iter()
                    .any(|(key, _)| key.as_text() == Some(CONTESTED));
                let unique = index_value_map.iter().fold(false, |unique, (key, value)| {
                    match (key.as_text(), value) {
                        (Some(UNIQUE), Value::Bool(declared)) => *declared,
                        _ => unique,
                    }
                });
                let base_index_fee = if contested {
                    fee_version.document_type_base_contested_index_registration_fee
                } else if unique {
                    fee_version.document_type_base_unique_index_registration_fee
                } else {
                    fee_version.document_type_base_non_unique_index_registration_fee
                };
                cost = cost.saturating_add(base_index_fee);
            }
        }

        for token_config in self.tokens().values() {
            cost = cost.saturating_add(fee_version.token_registration_fee);

            if token_config
                .distribution_rules()
                .perpetual_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_perpetual_distribution_fee);
            }

            if token_config
                .distribution_rules()
                .pre_programmed_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_pre_programmed_distribution_fee);
            }

            if token_config
                .distribution_rules()
                .once_per_identity_distribution()
                .is_some()
            {
                cost =
                    cost.saturating_add(fee_version.token_uses_once_per_identity_distribution_fee);
            }
        }

        let keyword_cost = fee_version
            .search_keyword_fee
            .saturating_mul(self.keywords().len() as u64);

        cost.saturating_add(keyword_cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use crate::prelude::DataContract;
    use crate::tests::fixtures::get_dashpay_contract_fixture;
    use platform_value::string_encoding::Encoding;
    use platform_value::Identifier;
    use platform_version::TryFromPlatformVersioned;
    use serde_json::json;

    fn serialized(contract: DataContract) -> DataContractInSerializationFormat {
        DataContractInSerializationFormat::try_from_platform_versioned(
            contract,
            PlatformVersion::latest(),
        )
        .expect("expected the serialization format")
    }

    /// A contract whose document type `item` has a contested index and one that is neither
    /// contested nor unique, and whose document type `tag` has a unique one.
    fn contract_with_every_kind_of_index() -> DataContract {
        let label = json!({ "type": "string", "maxLength": 20, "position": 0 });
        let other = json!({ "type": "string", "maxLength": 20, "position": 1 });
        let value = platform_value::to_value(json!({
            "$formatVersion": "1",
            "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": {
                "item": {
                    "type": "object",
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "properties": { "label": label, "other": other },
                    "required": ["label", "other"],
                    "indices": [
                        {
                            "name": "byLabel",
                            "properties": [{ "label": "asc" }],
                            "unique": true,
                            "contested": {
                                "fieldMatches": [
                                    { "field": "label", "regexPattern": "^[a-z]{3,9}$" }
                                ],
                                "resolution": 0
                            }
                        },
                        { "name": "byOther", "properties": [{ "other": "asc" }] }
                    ],
                    "additionalProperties": false
                },
                "tag": {
                    "type": "object",
                    "properties": { "label": label },
                    "required": ["label"],
                    "indices": [
                        { "name": "byLabel", "properties": [{ "label": "asc" }], "unique": true }
                    ],
                    "additionalProperties": false
                }
            }
        }))
        .expect("the contract should convert");
        DataContract::from_value(value, true, PlatformVersion::latest())
            .expect("the contract should parse")
    }

    #[test]
    fn should_bill_a_contract_that_registers_as_generation_2_does() {
        let platform_version = PlatformVersion::latest();
        for contract in [
            contract_with_every_kind_of_index(),
            get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
                .data_contract_owned(),
        ] {
            let parsed = contract.registration_cost_v2(platform_version);
            let serialized = serialized(contract);
            assert_eq!(serialized.registration_cost_v3(platform_version), parsed);
            assert_eq!(serialized.registration_cost_v2(platform_version), parsed);
        }
    }

    /// Generation 2 parses the index, which compiles its field matches, and bills nothing for
    /// one that does not compile. Generation 3 compiles none and bills the index all the same.
    #[test]
    fn should_bill_a_contested_index_without_compiling_its_field_matches() {
        let platform_version = PlatformVersion::latest();
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut contract = serialized(contract_with_every_kind_of_index());
        let schemas = contract
            .document_schemas_mut()
            .expect("expected a transport format's schemas");
        let field_match = schemas
            .get_mut("item")
            .and_then(|schema| schema.get_optional_value_mut("indices").ok().flatten())
            .and_then(|indices| indices.as_array_mut())
            .and_then(|indices| indices.first_mut())
            .and_then(|index| index.get_optional_value_mut("contested").ok().flatten())
            .and_then(|contested| {
                contested
                    .get_optional_value_mut("fieldMatches")
                    .ok()
                    .flatten()
            })
            .and_then(|field_matches| field_matches.as_array_mut())
            .and_then(|field_matches| field_matches.first_mut())
            .expect("expected the contested index's field match");
        field_match
            .set_value("regexPattern", Value::Text("(".to_string()))
            .expect("expected to replace the pattern");

        let unique_and_plain = fee_version.base_contract_registration_fee
            + 2 * fee_version.document_type_registration_fee
            + fee_version.document_type_base_unique_index_registration_fee
            + fee_version.document_type_base_non_unique_index_registration_fee;
        assert_eq!(
            contract.registration_cost_v2(platform_version),
            unique_and_plain
        );
        assert_eq!(
            contract.registration_cost_v3(platform_version),
            unique_and_plain + fee_version.document_type_base_contested_index_registration_fee
        );
    }
}
