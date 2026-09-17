use std::collections::BTreeSet;

use indexmap::IndexMap;

use crate::consensus::basic::data_contract::ContestedIndexInvalidParametersError;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::property::{DocumentProperty, DocumentPropertyType};
use crate::ProtocolError;

/// Generation 0 of the supported contested parameter rules, checked in this
/// order and each naming the index and the offending parameter:
///
/// 1. Every index property is a top-level user property: no dotted path and
///    no `$` system property. The vote poll key is built with a flat lookup
///    of the document's properties while the contested tree walker resolves
///    the same names through the document type, so a nested or system
///    property gives the two an empty or different key and every create on
///    the contest fails inside the node.
/// 2. Every index property is required. The contested tree cannot key a
///    null: an absent property reaches the walker as an empty key.
/// 3. Every index property is stored, that is not `transient`. A transient
///    property is stripped from the document before it is written, so the
///    contest is classified and the vote poll keyed from the submitted value
///    while the contested tree walker reads the stored document and finds
///    nothing: the same empty key as an absent property, on every create.
/// 4. Every `fieldMatches` field names a property of the index. A match on
///    another property lets two documents with equal index values take
///    different paths (one a contender, one an ordinary unique insert), and
///    the later award then collides in the unique index while the block
///    executes.
/// 5. Every matched property is a string. A regex match on any other type
///    never matches, so the index silently degrades to a plain unique index
///    and the contest can never start.
///
/// An index without a `contested` declaration passes; a contested index
/// without `fieldMatches` (always contested) is supported.
#[inline(always)]
pub(super) fn validate_contested_index_parameters_v0(
    document_type_name: &str,
    index: &Index,
    flattened_document_properties: &IndexMap<String, DocumentProperty>,
    required_fields: &BTreeSet<String>,
) -> Result<(), ProtocolError> {
    let Some(contested_index) = index.contested_index.as_ref() else {
        return Ok(());
    };

    let reject = |reason: String| -> Result<(), ProtocolError> {
        Err(ProtocolError::ConsensusError(Box::new(
            ContestedIndexInvalidParametersError::new(
                document_type_name.to_string(),
                index.name.clone(),
                reason,
            )
            .into(),
        )))
    };

    for property in &index.properties {
        let name = property.name.as_str();
        if name.starts_with('$') {
            return reject(format!(
                "index property '{name}' is a system property; contested index properties \
                 must be user-defined"
            ));
        }
        if name.contains('.') {
            return reject(format!(
                "index property '{name}' is nested; contested index properties must be \
                 top-level"
            ));
        }
        if !required_fields.contains(name) {
            return reject(format!(
                "index property '{name}' is optional; contested index properties must be \
                 listed in required"
            ));
        }
        if flattened_document_properties
            .get(name)
            .is_some_and(|property| property.transient)
        {
            return reject(format!(
                "index property '{name}' is transient; contested index properties must be \
                 stored with the document"
            ));
        }
    }

    for field in contested_index.field_matches.keys() {
        if !index
            .properties
            .iter()
            .any(|property| property.name == *field)
        {
            return reject(format!(
                "field match '{field}' does not name a property of the index"
            ));
        }
        match flattened_document_properties.get(field) {
            Some(DocumentProperty {
                property_type: DocumentPropertyType::String(_),
                ..
            }) => {}
            Some(property) => {
                return reject(format!(
                    "field match '{field}' names a property of type '{}'; only string \
                     properties can be matched",
                    property.property_type.name()
                ));
            }
            None => {
                return reject(format!(
                    "field match '{field}' names a property the document type does not define"
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use crate::ProtocolError;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Parses a DPNS-shaped document type under full validation at the
    /// latest version, so the contested index goes through this generation.
    ///
    /// Properties: four top-level strings and an integer, plus a nested
    /// identifier under `records`. `required` is the caller's, so a test can
    /// leave a property optional.
    fn parse(indices: Value, required: Value) -> Result<DocumentType, ProtocolError> {
        parse_with_transient(indices, required, platform_value!([]))
    }

    /// [`parse`] with the caller's `transient` list as well.
    fn parse_with_transient(
        indices: Value,
        required: Value,
        transient: Value,
    ) -> Result<DocumentType, ProtocolError> {
        let platform_version = PlatformVersion::latest();
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "label": {"type": "string", "position": 0, "maxLength": 63},
                "normalizedLabel": {"type": "string", "position": 1, "maxLength": 63},
                "normalizedParentDomainName": {"type": "string", "position": 2, "maxLength": 63},
                "priority": {"type": "integer", "position": 3},
                "records": {
                    "type": "object",
                    "position": 4,
                    "properties": {
                        "identity": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "position": 0,
                            "contentMediaType": "application/x.dash.dpp.identifier"
                        }
                    },
                    "minProperties": 1,
                    "additionalProperties": false
                },
            },
            "indices": indices,
            "required": required,
            "transient": transient,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on this platform version");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "domain",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            true,
            &mut vec![],
            platform_version,
        )
    }

    fn all_required() -> Value {
        platform_value!([
            "label",
            "normalizedLabel",
            "normalizedParentDomainName",
            "priority",
            "records"
        ])
    }

    /// Unwraps the rejection this generation produces and returns its reason.
    fn rejection_reason(result: Result<DocumentType, ProtocolError>, index_name: &str) -> String {
        let error = result.expect_err("expected the contested declaration to be rejected");
        let ProtocolError::ConsensusError(consensus_error) = error else {
            panic!("expected a consensus error, got {error:?}");
        };
        let ConsensusError::BasicError(BasicError::ContestedIndexInvalidParametersError(error)) =
            *consensus_error
        else {
            panic!("expected ContestedIndexInvalidParametersError, got {consensus_error:?}");
        };
        assert_eq!(error.document_type(), "domain");
        assert_eq!(error.index_name(), index_name);
        error.reason().to_string()
    }

    #[test]
    fn should_accept_a_dpns_shaped_contested_index() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "normalizedLabel", "regexPattern": "^[a-zA-Z01]{3,19}$"}],
                "resolution": 0,
                "description": "short names are contested"
            }
        }]);

        parse(indices, all_required()).expect("the DPNS declaration is supported");
    }

    #[test]
    fn should_accept_an_always_contested_index() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {"resolution": 0}
        }]);

        parse(indices, all_required())
            .expect("a contested index without field matches is supported");
    }

    #[test]
    fn should_reject_a_nested_contested_index_property() {
        let indices = platform_value!([{
            "name": "identityId",
            "properties": [{"records.identity": "asc"}],
            "unique": true,
            "contested": {"resolution": 0}
        }]);

        let reason = rejection_reason(parse(indices, all_required()), "identityId");

        assert_eq!(
            reason,
            "index property 'records.identity' is nested; contested index properties must be \
             top-level"
        );
    }

    #[test]
    fn should_reject_a_system_contested_index_property() {
        let indices = platform_value!([{
            "name": "owner",
            "properties": [{"$ownerId": "asc"}],
            "unique": true,
            "contested": {"resolution": 0}
        }]);

        let reason = rejection_reason(parse(indices, all_required()), "owner");

        assert_eq!(
            reason,
            "index property '$ownerId' is a system property; contested index properties must \
             be user-defined"
        );
    }

    #[test]
    fn should_reject_an_optional_contested_index_property() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {"resolution": 0}
        }]);
        let required =
            platform_value!(["label", "normalizedParentDomainName", "priority", "records"]);

        let reason = rejection_reason(parse(indices, required), "parentNameAndLabel");

        assert_eq!(
            reason,
            "index property 'normalizedLabel' is optional; contested index properties must be \
             listed in required"
        );
    }

    /// A required property may also be transient (DPNS's `preorderSalt` is both), and a
    /// transient property is stripped before the document is stored, so `required` alone
    /// does not prove the contested tree walker will find it.
    #[test]
    fn should_reject_a_transient_contested_index_property() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "normalizedLabel", "regexPattern": "^[a-zA-Z01]{3,19}$"}],
                "resolution": 0
            }
        }]);

        let reason = rejection_reason(
            parse_with_transient(
                indices,
                all_required(),
                platform_value!(["normalizedLabel"]),
            ),
            "parentNameAndLabel",
        );

        assert_eq!(
            reason,
            "index property 'normalizedLabel' is transient; contested index properties must \
             be stored with the document"
        );
    }

    /// A transient property outside the contested index (the DPNS shape, where
    /// `preorderSalt` is required and transient) is not the contest's concern.
    #[test]
    fn should_accept_a_transient_property_outside_the_contested_index() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {"resolution": 0}
        }]);

        parse_with_transient(indices, all_required(), platform_value!(["label"]))
            .expect("a transient property the index does not use is supported");
    }

    #[test]
    fn should_reject_a_field_match_on_a_property_outside_the_index() {
        let indices = platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "label", "regexPattern": "^[a-z]{3,19}$"}],
                "resolution": 0
            }
        }]);

        let reason = rejection_reason(parse(indices, all_required()), "parentNameAndLabel");

        assert_eq!(
            reason,
            "field match 'label' does not name a property of the index"
        );
    }

    #[test]
    fn should_reject_a_field_match_on_a_non_string_index_property() {
        let indices = platform_value!([{
            "name": "labelAndPriority",
            "properties": [{"normalizedLabel": "asc"}, {"priority": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "priority", "regexPattern": "^1$"}],
                "resolution": 0
            }
        }]);

        let reason = rejection_reason(parse(indices, all_required()), "labelAndPriority");

        assert_eq!(
            reason,
            "field match 'priority' names a property of type 'i64'; only string properties can \
             be matched"
        );
    }
}
