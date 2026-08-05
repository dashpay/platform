use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DocumentTypeRef<'_> {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Validate configuration
        let result = self.validate_config(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate indices
        let result = self
            .index_structure()
            .validate_update(self.name(), new_document_type.index_structure());

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate that no byte array property changes its on-disk encoding
        let result = self.validate_byte_array_encoding_stability(new_document_type);

        if !result.is_valid() {
            return Ok(result);
        }

        // Validate schema compatibility
        self.validate_schema(new_document_type, platform_version)
    }
}

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn doc_type_with_indices(indices: Value, platform_version: &PlatformVersion) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
                "b": {"type": "string", "position": 1, "maxLength": 60_u32},
                "c": {"type": "string", "position": 2, "maxLength": 60_u32},
            },
            "indices": indices,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("failed to create document type")
    }

    // Pins the frozen v0 behavior that motivated v1 (protocol v14): the
    // `IndexLevel` tree numbers its levels with a counter that follows the
    // iteration order of `indices` — a BTreeMap keyed by index NAME — so the
    // outcome of a semantically identical index addition depends on where the
    // new index's name sorts relative to existing ones.
    //
    // Old contract: "j" on [c], "k" on [a, b]. Adding an index on [a] (a
    // prefix of "k", terminating at an existing tree level):
    // - named "i" (sorts first): every level is renumbered, so the
    //   identifier-equality subset check rejects with an opaque
    //   "a -> Invalid path".
    // - named "z" (sorts last): the numbering is unchanged, the tree
    //   comparison passes, and the update falls through to the JSON-schema
    //   compatibility check, which hard-errors because no compatibility rule
    //   exists for the `indices` keyword.
    //
    // v0 stays byte-for-byte at this behavior for replay of protocol
    // versions <= 13; v1 replaces the tree comparison with a name-keyed
    // index-definition comparison.
    #[test]
    fn v0_index_addition_outcome_depends_on_index_name_sort_order() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 must exist");

        let old = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let new_early_name = doc_type_with_indices(
            platform_value!([
                {"name": "i", "properties": [{"a": "asc"}]},
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
            ]),
            platform_version,
        );

        let new_late_name = doc_type_with_indices(
            platform_value!([
                {"name": "j", "properties": [{"c": "asc"}]},
                {"name": "k", "properties": [{"a": "asc"}, {"b": "asc"}]},
                {"name": "z", "properties": [{"a": "asc"}]},
            ]),
            platform_version,
        );

        let early_result = old
            .as_ref()
            .validate_update(new_early_name.as_ref(), platform_version)
            .expect("early-name addition should produce a validation result");

        assert_matches!(
            early_result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
            )] if e.index_path() == "a -> Invalid path"
        );

        // The identical addition under a late-sorting name passes the tree
        // comparison and instead hard-errors in the schema compatibility
        // check ("schema keyword 'indices' ... is not supported").
        old.as_ref()
            .validate_update(new_late_name.as_ref(), platform_version)
            .expect_err("late-name addition should error in schema compatibility");
    }

    /// The ranking axes are index-level, so `validate_config` — which covers
    /// the *doctype*-level count / sum flags just above — is deliberately not
    /// where they are enforced. They ride the index-structure comparison
    /// (`IndexLevel::validate_update`) that `validate_update_v0` runs right
    /// after the config check. These tests exercise the PUBLIC dispatcher so
    /// that routing is pinned, not just the helper in isolation.
    mod validate_update_ranked_indices {
        use super::*;
        use std::collections::BTreeMap;

        /// `review` doctype, one averageable index over `restaurantId`, with
        /// `rankedAverageable` set to the supplied value.
        fn document_type_with_ranked_index(
            ranked_averageable: bool,
            platform_version: &PlatformVersion,
        ) -> DocumentType {
            let schema = platform_value!({
                "type": "object",
                "properties": {
                    // 32 rather than the generic 63-character index limit:
                    // an index declaring a ranking axis bounds its group key
                    // more tightly (59 characters on the Avg axis), and both
                    // halves of these tests have to build the same doctype
                    // shape with only `rankedAverageable` differing.
                    "restaurantId": {
                        "type": "string",
                        "maxLength": 32,
                        "position": 0,
                    },
                    "grade": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 100,
                        "position": 1,
                    },
                },
                "required": ["restaurantId", "grade"],
                "additionalProperties": false,
                "indices": [{
                    "name": "byRestaurant",
                    "properties": [{ "restaurantId": "asc" }],
                    "averageable": "grade",
                    "rangeAverageable": true,
                    "rankedAverageable": ranked_averageable,
                }],
            });

            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create a default config");

            DocumentType::try_from_schema(
                Identifier::random(),
                1,
                config.version(),
                "review",
                schema,
                None,
                &BTreeMap::new(),
                &config,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("failed to create document type")
        }

        #[test]
        fn should_return_invalid_result_when_ranked_averageable_is_changed() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(false, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), platform_version)
                .expect("validate_update should not error");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    crate::consensus::basic::BasicError::DataContractInvalidIndexDefinitionUpdateError(e)
                )] if e.index_path() == "restaurantId -> (ranked_averageable: false -> true)"
            );
        }

        #[test]
        fn should_pass_when_ranked_averageable_is_unchanged() {
            let platform_version = PlatformVersion::latest();
            let old = document_type_with_ranked_index(true, platform_version);
            let new = document_type_with_ranked_index(true, platform_version);

            let result = old
                .as_ref()
                .validate_update(new.as_ref(), platform_version)
                .expect("validate_update should not error");

            assert!(
                result.is_valid(),
                "an unchanged ranked index must not be rejected, got {:?}",
                result.errors
            );
        }
    }
}
