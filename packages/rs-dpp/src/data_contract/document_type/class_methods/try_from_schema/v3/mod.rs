use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::{TokenConfiguration, TokenContractPosition};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

impl DocumentType {
    /// V3 delegates parsing to the V2 parser, then rejects the
    /// self-contradictory `documentsKeepHistory: true` + `canBeDeleted: true`
    /// combination during full validation.
    ///
    /// The combination is contradictory because rs-drive unconditionally
    /// refuses to delete a document whose type keeps history
    /// (`force_delete_document_for_contract_operations_v0` returns
    /// `InvalidDeletionOfDocumentThatKeepsHistory`), so `canBeDeleted: true`
    /// advertises a capability the storage layer will always reject. Catching
    /// it at parse time turns the contradiction into a clean validation error
    /// at contract creation, before any delete is attempted. Mirrors the
    /// existing cross-flag rule for
    /// `ContestedUniqueIndexOnMutableDocumentTypeError`.
    ///
    /// The check lives in a new `try_from_schema` version (active from
    /// protocol version 13, via `CONTRACT_VERSIONS_V5`) rather than inside the
    /// V2 parser because V2 is shared with protocol version 12, which is
    /// released and therefore consensus-frozen — contracts accepted at v12
    /// must replay identically.
    ///
    /// Gated by `full_validation` so already-deployed contradictory contracts
    /// (e.g. testnet `5CBPiadGmx3Zsjc26g5onopcx7pdxHPbrRAUD2T2yAbC` document
    /// type `note`) continue to load when re-parsed at v13+ — the drive-abci
    /// delete-transition guard turns their deletes into normal invalid (paid)
    /// transitions instead of internal errors at that layer.
    ///
    /// Uses `consensus_or_protocol_data_contract_error` so that with the
    /// `validation` feature this surfaces as `ProtocolError::ConsensusError`;
    /// drive-abci's `transform_into_action_v0` only converts that variant
    /// into an invalid (paid) transition with a bump action — a bare
    /// `ProtocolError::DataContractError` would propagate as an internal
    /// execution error in validator mode.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::data_contract::document_type::class_methods) fn try_from_schema_v3(
        data_contract_id: Identifier,
        data_contract_system_version: u16,
        contract_config_version: u16,
        name: &str,
        schema: Value,
        schema_defs: Option<&BTreeMap<String, Value>>,
        token_configurations: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        data_contact_config: &DataContractConfig,
        full_validation: bool,
        validation_operations: &mut impl Extend<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let document_type = DocumentType::try_from_schema_v2(
            data_contract_id,
            data_contract_system_version,
            contract_config_version,
            name,
            schema,
            schema_defs,
            token_configurations,
            data_contact_config,
            full_validation,
            validation_operations,
            platform_version,
        )?;

        // The flags are read from the parsed result (not the raw schema) so
        // the check sees `canBeDeleted` resolved against the contract config
        // default (`true` when the key is omitted).
        if full_validation
            && document_type.documents_keep_history()
            && document_type.documents_can_be_deleted()
        {
            return Err(consensus_or_protocol_data_contract_error(
                DataContractError::InvalidContractStructure(format!(
                    "document type \"{}\" sets both `documentsKeepHistory: true` and \
                     `canBeDeleted: true`, but the storage layer unconditionally refuses to \
                     delete a document whose type keeps history. Set `canBeDeleted` to false or \
                     disable `documentsKeepHistory`.",
                    name,
                )),
            ));
        }

        Ok(document_type)
    }
}

#[cfg(test)]
mod tests {
    //! Regression tests for the `documentsKeepHistory` + `canBeDeleted`
    //! cross-flag rule added in `try_from_schema` v3 (protocol version 13).
    use super::*;
    use platform_value::platform_value;

    /// Parses through the public dispatcher at the given protocol version so
    /// the test exercises the same `try_from_schema` version routing consensus
    /// code uses (v3 at protocol version 13, v2 at 12).
    fn parse_at_version(
        schema: Value,
        protocol_version: u32,
        full_validation: bool,
    ) -> Result<DocumentType, ProtocolError> {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "test_doc",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            platform_version,
        )
    }

    fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
        parse_at_version(schema, 13, true)
    }

    fn keep_history_deletable_schema() -> Value {
        platform_value!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "maxLength": 50,
                    "position": 0,
                },
            },
            "additionalProperties": false,
            "documentsKeepHistory": true,
            "canBeDeleted": true,
        })
    }

    /// `documentsKeepHistory: true` + `canBeDeleted: true` is
    /// self-contradictory: rs-drive unconditionally refuses to delete
    /// a document whose type keeps history
    /// (`InvalidDeletionOfDocumentThatKeepsHistory`), so `canBeDeleted:
    /// true` advertises a capability the storage layer will always
    /// reject. The parser must reject the combination at contract
    /// creation time so an SDK user gets a clean validation error
    /// instead of the delete failing as an internal error at execution.
    ///
    /// With the `validation` feature enabled the rejection must surface
    /// as `ProtocolError::ConsensusError` (not bare
    /// `ProtocolError::DataContractError`) — drive-abci's
    /// `transform_into_action_v0` only turns the consensus variant into
    /// a clean invalid (paid) transition with a bump action; the
    /// data-contract-error variant propagates as an internal execution
    /// error in validator mode.
    #[test]
    fn doctype_keep_history_with_can_be_deleted_rejected() {
        let result = parse(keep_history_deletable_schema());
        assert!(
            result.is_err(),
            "documentsKeepHistory: true + canBeDeleted: true must be rejected"
        );
        let err = result.unwrap_err();
        let msg = format!("{:?}", err);
        assert!(
            msg.contains("documentsKeepHistory") && msg.contains("canBeDeleted"),
            "error must reference both documentsKeepHistory and canBeDeleted; got {msg}"
        );
        #[cfg(feature = "validation")]
        assert!(
            matches!(err, ProtocolError::ConsensusError(_)),
            "with `validation` feature the rejection must be ProtocolError::ConsensusError so \
             drive-abci's transform_into_action turns it into an invalid (paid) transition \
             with a bump action rather than propagating as an internal execution error; got \
             {err:?}"
        );
    }

    /// Omitting `canBeDeleted` exercises the contract-config default boundary:
    /// the latest config defaults it to `true`, so a keep-history document type
    /// remains contradictory and must be rejected during full validation.
    #[test]
    fn doctype_keep_history_with_can_be_deleted_omitted_rejected() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "maxLength": 50,
                    "position": 0,
                },
            },
            "additionalProperties": false,
            "documentsKeepHistory": true,
        });
        let result = parse(schema);
        assert!(
            result.is_err(),
            "omitted canBeDeleted must default to true and conflict with documentsKeepHistory"
        );
        let msg = format!("{:?}", result.unwrap_err());
        assert!(
            msg.contains("documentsKeepHistory") && msg.contains("canBeDeleted"),
            "error must reference both documentsKeepHistory and defaulted canBeDeleted; got {msg}"
        );
    }

    /// `documentsKeepHistory: true` + `canBeDeleted: true` is rejected
    /// ONLY when `full_validation: true`. With `full_validation: false`
    /// (the restore / migration / cache-warmup path) the same schema must
    /// parse cleanly so already-deployed contradictory contracts continue
    /// to load at v13+ — the drive-abci delete-transition guard turns
    /// their deletes into clean invalid (paid) transitions instead of
    /// rejecting them as internal errors at the contract-load layer.
    #[test]
    fn doctype_keep_history_with_can_be_deleted_accepted_without_full_validation() {
        let document_type = parse_at_version(keep_history_deletable_schema(), 13, false).expect(
            "documentsKeepHistory: true + canBeDeleted: true must be accepted when \
             full_validation: false so already-deployed contradictory contracts continue to load",
        );
        assert!(document_type.documents_keep_history());
        assert!(document_type.documents_can_be_deleted());
    }

    /// Protocol version 12 routes to `try_from_schema` v2, which has no
    /// cross-flag rule — the combination must stay accepted there even under
    /// full validation, because v12 is released and consensus-frozen:
    /// contracts accepted at v12 must replay identically.
    #[test]
    fn doctype_keep_history_with_can_be_deleted_accepted_at_protocol_version_12() {
        let document_type = parse_at_version(keep_history_deletable_schema(), 12, true).expect(
            "documentsKeepHistory: true + canBeDeleted: true must stay accepted at protocol \
             version 12 (consensus-frozen v2 parser) for replay compatibility",
        );
        assert!(document_type.documents_keep_history());
        assert!(document_type.documents_can_be_deleted());
    }

    /// Guard against an over-broad fix: `documentsKeepHistory: true` +
    /// `canBeDeleted: false` is consistent (the doctype is append-only)
    /// and must continue to parse cleanly. The sibling omitted-key regression
    /// covers the distinct default-`true` boundary and therefore expects
    /// rejection rather than acceptance.
    #[test]
    fn doctype_keep_history_with_can_be_deleted_false_accepted() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "maxLength": 50,
                    "position": 0,
                },
            },
            "additionalProperties": false,
            "documentsKeepHistory": true,
            "canBeDeleted": false,
        });
        let document_type = parse(schema).expect(
            "documentsKeepHistory: true + canBeDeleted: false is consistent and must parse",
        );
        assert!(document_type.documents_keep_history());
        assert!(!document_type.documents_can_be_deleted());
    }

    /// Symmetric guard: `canBeDeleted: true` on a non-keep-history
    /// doctype must continue to parse cleanly. Catches a predicate that
    /// triggers on `canBeDeleted: true` alone instead of the AND.
    #[test]
    fn doctype_can_be_deleted_without_keep_history_accepted() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "maxLength": 50,
                    "position": 0,
                },
            },
            "additionalProperties": false,
            "canBeDeleted": true,
        });
        let document_type = parse(schema)
            .expect("canBeDeleted: true without documentsKeepHistory must parse cleanly");
        assert!(!document_type.documents_keep_history());
        assert!(document_type.documents_can_be_deleted());
    }
}
