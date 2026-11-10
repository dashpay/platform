use crate::data_contract::update_values::DataContractUpdateValues;
use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionV1 {
    /// The registration cost of what this delta registers: its new and replaced document
    /// types (with their indexes), its new tokens and its added keywords, without the fee of
    /// registering a contract. See [`DataContractUpdateValues::registration_cost`].
    pub fn registration_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        DataContractUpdateValues::from(self).registration_cost(platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Setters;
    use crate::data_contract::accessors::v1::DataContractV1Setters;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
    use crate::data_contract::DataContract;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_value::platform_value;

    fn contract() -> DataContract {
        get_data_contract_fixture(None, 0, PlatformVersion::latest().protocol_version)
            .data_contract_owned()
    }

    fn delta_between(
        old_contract: &DataContract,
        new_contract: &DataContract,
    ) -> DataContractUpdateTransitionV1 {
        DataContractUpdateTransitionV1::from_contract_update(old_contract, new_contract, 1)
            .expect("expected the update to be expressible as a delta")
    }

    #[test]
    fn should_cost_nothing_to_register_nothing() {
        let old_contract = contract();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract.set_description(Some("a described contract".to_string()));

        assert_eq!(
            delta_between(&old_contract, &new_contract)
                .registration_cost(PlatformVersion::latest())
                .expect("expected a registration cost"),
            0
        );
    }

    /// A delta pays for what it registers and nothing else: the whole contract's registration
    /// cost rises by exactly the delta's.
    #[test]
    fn should_cost_what_the_registration_cost_of_the_contract_rises_by() {
        let platform_version = PlatformVersion::latest();
        let fees = &platform_version.fee_version.data_contract_registration;
        let old_contract = contract();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract
            .set_document_schema(
                "note",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "label": {"type": "string", "maxLength": 63, "position": 0}
                    },
                    "indices": [
                        {"name": "byLabel", "properties": [{"label": "asc"}], "unique": true},
                        {"name": "byOwner", "properties": [{"$ownerId": "asc"}]}
                    ],
                    "additionalProperties": false
                }),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add a document type");
        new_contract.set_keywords(vec!["alpha".to_string(), "beta".to_string()]);

        let cost = delta_between(&old_contract, &new_contract)
            .registration_cost(platform_version)
            .expect("expected a registration cost");

        assert_eq!(
            cost,
            fees.document_type_registration_fee
                + fees.document_type_base_unique_index_registration_fee
                + fees.document_type_base_non_unique_index_registration_fee
                + 2 * fees.search_keyword_fee
        );
        assert_eq!(
            cost,
            new_contract
                .registration_cost(platform_version)
                .expect("expected a registration cost")
                - old_contract
                    .registration_cost(platform_version)
                    .expect("expected a registration cost")
        );
    }

    /// A replaced document type is registered again, as an update carrying the whole contract
    /// registers every type again.
    #[test]
    fn should_cost_a_replaced_document_type_as_a_registered_one() {
        let platform_version = PlatformVersion::latest();
        let fees = &platform_version.fee_version.data_contract_registration;
        let old_contract = contract();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        let mut replaced_schema = old_contract
            .document_schemas()
            .get("niceDocument")
            .cloned()
            .cloned()
            .expect("expected the fixture's niceDocument type");
        replaced_schema
            .set_value(
                "description",
                platform_value::Value::Text("a nicer document".to_string()),
            )
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

        assert_eq!(
            delta_between(&old_contract, &new_contract)
                .registration_cost(platform_version)
                .expect("expected a registration cost"),
            fees.document_type_registration_fee
        );
    }
}
