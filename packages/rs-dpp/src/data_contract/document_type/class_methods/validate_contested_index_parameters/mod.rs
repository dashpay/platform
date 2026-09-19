use std::collections::BTreeSet;

use indexmap::IndexMap;
use platform_version::version::PlatformVersion;

use crate::data_contract::document_type::index::Index;
use crate::data_contract::document_type::property::DocumentProperty;
use crate::ProtocolError;

mod v0;

/// Checks that a contested index declares only parameters the native contest
/// machinery can honour. Called from the shared document-type parser core for
/// every index parsed under full validation; an index without a `contested`
/// declaration passes untouched.
///
/// Contracts parameterize a contest only through the native `contested`
/// declaration (`fieldMatches`, `resolution`, `description`). The parser
/// already rejects malformed declarations (an invalid regex, an unknown
/// resolution, a contested index that is not unique). What it does not know
/// is whether the declared parameters can be honoured once a contest starts:
/// the vote poll key, the contested tree path, the contest detection and the
/// award all read the index properties and the matched properties through
/// the document, and each of the rules generation 0 enforces closes a way for
/// that to break on the node rather than at registration.
///
/// Versioned on `dpp.validation.document_type.validate_contested_index_parameters`.
/// `None` selects the behaviour of the versions that predate the check: every
/// declaration the parser accepts is accepted, so stored contracts and
/// history validated before the check existed are never re-judged.
///
/// # Parameters
/// * `document_type_name`: The name of the document type the index belongs to.
/// * `index`: The parsed index.
/// * `flattened_document_properties`: Every user property of the document type keyed by
///   its dotted path.
/// * `required_fields`: Every required property path of the document type.
/// * `platform_version`: The platform version to select the correct function version to run.
///
/// # Returns
/// * `Ok(())` if the index is not contested or declares only supported parameters.
/// * `Err(ProtocolError::ConsensusError(ContestedIndexInvalidParametersError))` naming the
///   index and the offending parameter otherwise.
/// * `Err(ProtocolError::UnknownVersionMismatch)` if the platform version selects an unknown
///   generation.
pub(crate) fn validate_contested_index_parameters(
    document_type_name: &str,
    index: &Index,
    flattened_document_properties: &IndexMap<String, DocumentProperty>,
    required_fields: &BTreeSet<String>,
    platform_version: &PlatformVersion,
) -> Result<(), ProtocolError> {
    match platform_version
        .dpp
        .validation
        .document_type
        .validate_contested_index_parameters
    {
        None => Ok(()),
        Some(0) => v0::validate_contested_index_parameters_v0(
            document_type_name,
            index,
            flattened_document_properties,
            required_fields,
        ),
        Some(version) => Err(ProtocolError::UnknownVersionMismatch {
            method: "validate_contested_index_parameters".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    //! The gate, seen through the real document-type parser: the same
    //! declarations parse at protocol version 14 (table slot `None`) and are
    //! rejected at the latest version, and the latest version still parses
    //! them when full validation is off, which is how stored contracts are
    //! loaded.

    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use crate::ProtocolError;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// A DPNS-shaped document type whose contested index is the caller's.
    fn parse(
        indices: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentType, ProtocolError> {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "label": {"type": "string", "position": 0, "maxLength": 63},
                "normalizedLabel": {"type": "string", "position": 1, "maxLength": 63},
                "normalizedParentDomainName": {"type": "string", "position": 2, "maxLength": 63},
                "priority": {"type": "integer", "position": 3},
            },
            "indices": indices,
            "required": ["label", "normalizedLabel", "normalizedParentDomainName", "priority"],
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
            full_validation,
            &mut vec![],
            platform_version,
        )
    }

    /// A field match on `label`, which is not a property of the index.
    fn field_match_outside_the_index() -> Value {
        platform_value!([{
            "name": "parentNameAndLabel",
            "properties": [{"normalizedParentDomainName": "asc"}, {"normalizedLabel": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "label", "regexPattern": "^[a-z]{3,19}$"}],
                "resolution": 0
            }
        }])
    }

    #[test]
    fn should_accept_an_unsupported_declaration_before_the_check_existed() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");

        parse(field_match_outside_the_index(), true, platform_version)
            .expect("protocol version 14 accepts every declaration the parser accepts");
    }

    #[test]
    fn should_reject_an_unsupported_declaration_at_the_latest_version() {
        let platform_version = PlatformVersion::latest();

        let error = parse(field_match_outside_the_index(), true, platform_version)
            .expect_err("the latest version rejects a field match outside the index");

        assert_matches!(
            error,
            ProtocolError::ConsensusError(consensus_error)
                if matches!(
                    *consensus_error,
                    ConsensusError::BasicError(BasicError::ContestedIndexInvalidParametersError(_))
                )
        );
    }

    #[test]
    fn should_not_re_judge_a_stored_contract_at_the_latest_version() {
        let platform_version = PlatformVersion::latest();

        parse(field_match_outside_the_index(), false, platform_version)
            .expect("a contract loaded from state is parsed without full validation");
    }
}
