use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::property::{ByteArrayPropertySizes, DocumentPropertyType};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DocumentTypeRef<'_> {
    /// `validate_update` feature version 1.
    ///
    /// Identical to v0 (config + index + schema-compatibility) plus an extra
    /// check that no byte array property changes its on-disk encoding. v0 is left
    /// bit-exact for already-released protocol versions; this tighter rule is
    /// selected only by the protocol version that pins `validate_update` to 1, so
    /// it activates atomically at that protocol upgrade rather than on binary
    /// rollout.
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = self.validate_update_v0(new_document_type, platform_version)?;

        if !result.is_valid() {
            return Ok(result);
        }

        Ok(self.validate_byte_array_encoding_stability(new_document_type))
    }

    /// A byte array property whose `minItems == maxItems` is serialized as raw,
    /// fixed-length bytes with no length prefix; any other size bounds make it
    /// serialized with a variable-length (varint) length prefix. Crossing that
    /// boundary -- or changing the fixed length itself -- silently changes the
    /// on-disk layout of every already-stored document, so re-decoding old bytes
    /// against the new type misreads them. JSON-schema compatibility treats
    /// widening/removing `maxItems` as compatible, so this layout invariant must
    /// be enforced separately.
    fn validate_byte_array_encoding_stability(
        &self,
        new_document_type: DocumentTypeRef,
    ) -> SimpleConsensusValidationResult {
        // Mirror the encoder/decoder exactly (see `encode_value_ref_with_size`):
        // the raw, no-length-prefix path is used ONLY when BOTH bounds are present
        // and equal. Any other shape -- including an omitted `minItems` (`None`) --
        // is varint length-prefixed, so an implicit `minItems: 0` must NOT be
        // treated as fixed-length here or this guard would diverge from the actual
        // on-disk layout. `Some(n)` => fixed raw encoding of length `n`; `None` =>
        // variable (varint length-prefixed) encoding.
        fn fixed_length(sizes: &ByteArrayPropertySizes) -> Option<u16> {
            match (sizes.min_size, sizes.max_size) {
                (Some(min), Some(max)) if min == max => Some(min),
                _ => None,
            }
        }

        let new_properties = new_document_type.flattened_properties();

        for (path, old_property) in self.flattened_properties() {
            let DocumentPropertyType::ByteArray(old_sizes) = &old_property.property_type else {
                continue;
            };

            let Some(new_property) = new_properties.get(path) else {
                continue;
            };

            let DocumentPropertyType::ByteArray(new_sizes) = &new_property.property_type else {
                continue;
            };

            if fixed_length(old_sizes) != fixed_length(new_sizes) {
                return SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.data_contract_id(),
                        self.name(),
                        format!(
                            "document type can not change the byte array encoding of property \
                             '{}': changing its size bounds from (minItems: {:?}, maxItems: {:?}) \
                             to (minItems: {:?}, maxItems: {:?}) alters the on-disk layout of \
                             existing documents",
                            path,
                            old_sizes.min_size,
                            old_sizes.max_size,
                            new_sizes.min_size,
                            new_sizes.max_size,
                        ),
                    )
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier};
    use std::collections::BTreeMap;

    fn document_type_with_byte_array(
        byte_array: platform_value::Value,
        platform_version: &PlatformVersion,
    ) -> DocumentType {
        let schema = platform_value!({
            "type": "object",
            "properties": { "blob": byte_array },
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::random(),
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

    /// Exercises the PUBLIC `validate_update` dispatcher so a missing match arm or
    /// stale `validate_update` feature-version constant fails the test. The latest
    /// protocol version selects feature version 1 (the byte-array check).
    fn validate_update_latest(
        old_ba: platform_value::Value,
        new_ba: platform_value::Value,
    ) -> SimpleConsensusValidationResult {
        let platform_version = PlatformVersion::latest();
        let old = document_type_with_byte_array(old_ba, platform_version);
        let new = document_type_with_byte_array(new_ba, platform_version);
        old.as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error")
    }

    fn assert_rejected(old_ba: platform_value::Value, new_ba: platform_value::Value) {
        let result = validate_update_latest(old_ba, new_ba);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message().contains("byte array encoding")
        );
    }

    fn assert_accepted(old_ba: platform_value::Value, new_ba: platform_value::Value) {
        let result = validate_update_latest(old_ba, new_ba);
        assert!(
            result.is_valid(),
            "expected the update to be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn rejects_widening_fixed_byte_array_max_items() {
        // The exact attack: a fixed (raw, no length prefix) 32-byte field widened
        // to min 32 / max 64 flips it to the varint length-prefixed encoding,
        // making every already-stored document undecodable.
        assert_rejected(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":64,"position":0}),
        );
    }

    #[test]
    fn rejects_removing_max_items_from_fixed_byte_array() {
        // Removing `maxItems` turns a fixed (raw, no length prefix) byte array
        // into a variable (varint length-prefixed) one, so it must be rejected.
        assert_rejected(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            platform_value!({"type":"array","byteArray":true,"minItems":32,"position":0}),
        );
    }

    #[test]
    fn byte_array_check_rejects_fixed_size_change() {
        // A fixed-size change (32 -> 64) also alters the on-disk layout and is
        // rejected by the byte-array check. Through the public dispatcher this
        // case is additionally caught earlier by JSON-schema compatibility (a
        // minItems widening is itself incompatible), so we exercise the byte-array
        // check in isolation here to cover its fixed-size branch directly.
        let platform_version = PlatformVersion::latest();
        let old = document_type_with_byte_array(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            platform_version,
        );
        let new = document_type_with_byte_array(
            platform_value!({"type":"array","byteArray":true,"minItems":64,"maxItems":64,"position":0}),
            platform_version,
        );
        let result = old
            .as_ref()
            .validate_byte_array_encoding_stability(new.as_ref());
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))]
                if e.additional_message().contains("byte array encoding")
        );
    }

    #[test]
    fn accepts_unchanged_fixed_byte_array() {
        assert_accepted(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
        );
    }

    #[test]
    fn accepts_max_items_change_when_min_items_is_omitted() {
        // With `minItems` omitted (None) the encoder always uses the variable
        // (varint length-prefixed) path regardless of `maxItems` -- the raw path
        // requires BOTH bounds present and equal. So changing `maxItems` does not
        // change the on-disk encoding and must stay allowed. An implicit
        // `minItems: 0` is NOT fixed-length (it mirrors encode_value_ref_with_size).
        assert_accepted(
            platform_value!({"type":"array","byteArray":true,"maxItems":0,"position":0}),
            platform_value!({"type":"array","byteArray":true,"maxItems":1,"position":0}),
        );
    }

    #[test]
    fn accepts_widening_already_variable_byte_array() {
        // Variable-length on both sides: the on-disk encoding does not change, so
        // widening the bound stays allowed.
        assert_accepted(
            platform_value!({"type":"array","byteArray":true,"minItems":1,"maxItems":32,"position":0}),
            platform_value!({"type":"array","byteArray":true,"minItems":1,"maxItems":64,"position":0}),
        );
    }

    #[test]
    fn pre_activation_protocol_version_still_accepts_the_flip() {
        // Gating proof: under a protocol version that pins `validate_update` to
        // feature version 0 (no byte-array check), the same encoding flip is
        // accepted. This is why the rule must activate at a protocol version
        // boundary rather than on binary rollout.
        let platform_version = PlatformVersion::get(11).expect("protocol version 11 should exist");
        let old = document_type_with_byte_array(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":32,"position":0}),
            platform_version,
        );
        let new = document_type_with_byte_array(
            platform_value!({"type":"array","byteArray":true,"minItems":32,"maxItems":64,"position":0}),
            platform_version,
        );
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), platform_version)
            .expect("validate_update should not error");
        assert!(
            result.is_valid(),
            "pre-activation protocol version must NOT reject the flip, got {:?}",
            result.errors
        );
    }
}
