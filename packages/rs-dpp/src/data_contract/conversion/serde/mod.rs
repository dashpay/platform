//! Manual `Serialize` / `Deserialize` for the outer `DataContract` enum.
//!
//! # Critical-4: platform-version coupling (pinned by tests below)
//!
//! Both impls call `PlatformVersion::get_version_or_current_or_latest(None)`,
//! making serialization output *depend on a process-global thread-local-ish*
//! state — same DataContract value, different bytes if the active platform
//! version changes. This is by design: `DataContract` is a versioned enum
//! routed through `DataContractInSerializationFormat`, and the format depends
//! on the current platform.
//!
//! # Validation policy: validate by default
//!
//! The `Deserialize` impl runs **full schema validation** — it decodes the
//! wire form into `DataContractInSerializationFormat`, then converts it to a
//! `DataContract` with `full_validation = true`. So
//! `serde_json::from_value::<DataContract>(...)` / `from_str` reject a
//! structurally-decodable but schema-invalid contract: a `DataContract` you
//! can hold is a valid one.
//!
//! To reconstruct already-trusted data *without* re-validating it (e.g.
//! storage reads, round-trips of a known-good value), use the explicit
//! no-validation path
//! `DataContractJsonConversionMethodsV0::from_json(value, false, pv)` /
//! `DataContractValueConversionMethodsV0::from_value(value, false, pv)` — those
//! call `try_from_platform_versioned` directly and skip schema validation.
//!
//! **Why this is KEEP-AS-EXCEPTION**: the manual impls also inject the active
//! `PlatformVersion` (via `get_version_or_current_or_latest`), coupling the
//! output to process-global state. This is by design — `DataContract` is a
//! versioned enum routed through `DataContractInSerializationFormat`, and the
//! format depends on the current platform. The stateless alternatives are the
//! bincode storage path (`serialize_to_bytes_with_platform_version`) and, for
//! `platform_value` output, `DataContractValueConversionMethodsV0::to_value`,
//! which threads an explicit `PlatformVersion` (prefer it wherever one is in
//! hand — the global here can be mutated concurrently, e.g. by parallel tests
//! building platforms at older protocol versions).
//!
//! The `data_contract_serde_pins_critical_4` test module below pins this
//! behavior (validate-by-default + explicit `false` opt-out) so future
//! refactors can't silently change it.

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::DataContract;
use crate::version::PlatformVersionCurrentVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for DataContract {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let current_version = PlatformVersion::get_version_or_current_or_latest(None)
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        let data_contract_in_serialization_format: DataContractInSerializationFormat = self
            .try_into_platform_versioned(current_version)
            .map_err(|e: ProtocolError| serde::ser::Error::custom(format!("expected to be able to serialize data contract into its serialized version: {}", e)))?;
        data_contract_in_serialization_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataContract {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialization_format = DataContractInSerializationFormat::deserialize(deserializer)?;
        let current_version = PlatformVersion::get_version_or_current_or_latest(None)
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        // Full schema validation: a deserialized `DataContract` is a valid one.
        // To reconstruct already-trusted data without re-validating, use the
        // explicit `from_json`/`from_value(_, false, _)` path instead of serde.
        // See the module-level doc comment for the rationale.
        DataContract::try_from_platform_versioned(
            serialization_format,
            true,
            &mut vec![],
            current_version,
        )
        .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod data_contract_serde_pins_critical_4 {
    //! Behavior pins for Critical-4 (DataContract serde impurity).
    //!
    //! These tests don't fix anything — they snapshot the current behavior
    //! so a future refactor that quietly changes either the
    //! `PlatformVersion::get_current()` coupling or the hardcoded
    //! `full_validation = true` will fail loudly.
    //!
    //! See module-level doc above and the unification plan §3.0 Critical-4.
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::LATEST_PLATFORM_VERSION;

    /// PIN: `DataContract` round-trips through `serde_json` at the active
    /// platform version. Documents that `Serialize` / `Deserialize` are
    /// load-bearing for JSON-shape interchange (not just bincode).
    #[test]
    fn data_contract_round_trips_through_serde_json() {
        let created = get_data_contract_fixture(None, 0, 1);
        let original = created.data_contract().clone();

        let json = serde_json::to_value(&original).expect("serialize to json");
        let recovered: DataContract = serde_json::from_value(json).expect("deserialize from json");

        assert_eq!(original.id(), recovered.id());
        assert_eq!(original.owner_id(), recovered.owner_id());
        assert_eq!(original.version(), recovered.version());
    }

    /// PIN: `DataContract::serialize` produces the same wire shape as
    /// `DataContractInSerializationFormat::serialize`. This documents that
    /// the manual impl is a thin wrapper that injects
    /// `PlatformVersion::get_current()` and forwards to the format type —
    /// not a custom shape.
    #[test]
    fn data_contract_serialize_matches_serialization_format_at_current_version() {
        let created = get_data_contract_fixture(None, 0, 1);
        let original = created.data_contract().clone();

        let direct_json = serde_json::to_value(&original).expect("DataContract -> json");

        let format: DataContractInSerializationFormat = original
            .try_into_platform_versioned(LATEST_PLATFORM_VERSION)
            .expect("DataContract -> SerializationFormat at latest");
        let format_json = serde_json::to_value(&format).expect("SerializationFormat -> json");

        assert_eq!(
            direct_json, format_json,
            "DataContract::serialize should be byte-equivalent to \
             DataContractInSerializationFormat::serialize at the current \
             platform version. If this fails, the manual serde impl has \
             diverged from the format-routing pattern documented in the \
             module-level comment."
        );
    }

    /// PIN: `DataContract::deserialize` runs **full schema validation** — a
    /// deserialized `DataContract` is a valid one. The explicit
    /// `from_json(_, false, _)` path is the opt-out for reconstructing
    /// already-trusted data without re-validation.
    ///
    /// We exercise this with a structurally well-formed payload whose document
    /// schema is semantically invalid (an `indices` entry referencing a
    /// nonexistent property):
    ///
    /// - canonical `DataContract::deserialize` REJECTS it (validation runs).
    /// - explicit `from_json(_, false, _)` ACCEPTS it (validation skipped).
    /// - explicit `from_json(_, true, _)` REJECTS it (validation runs).
    ///
    /// If a future refactor flips canonical Deserialize back to not validating,
    /// this test fails loudly. See module-level doc above for the rationale.
    #[test]
    fn data_contract_deserialize_validates_by_default() {
        use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;

        // Build a valid contract, then mutate its JSON to make the schema
        // semantically invalid: declare an index over a property not in
        // the schema's `properties` map. Structurally well-formed JSON;
        // only schema validation catches the issue.
        let created = get_data_contract_fixture(None, 0, 1);
        let original = created.data_contract().clone();

        let mut json = serde_json::to_value(&original).expect("to_json");

        let document_schemas = json
            .get_mut("documentSchemas")
            .and_then(|v| v.as_object_mut())
            .expect("documentSchemas object");
        let (_, first_schema) = document_schemas
            .iter_mut()
            .next()
            .expect("at least one document schema");
        let schema_obj = first_schema.as_object_mut().expect("schema is object");
        schema_obj.insert(
            "indices".to_string(),
            serde_json::json!([
                {
                    "name": "invalid_idx",
                    "properties": [{"definitelyDoesNotExist": "asc"}],
                    "unique": false,
                }
            ]),
        );

        // Format-level deserialize succeeds (never validated).
        let _: DataContractInSerializationFormat = serde_json::from_value(json.clone())
            .expect("format-level deserialize should accept structurally-valid input");

        // PIN: canonical Deserialize REJECTS the invalid schema (validates).
        let canonical_result: Result<DataContract, _> = serde_json::from_value(json.clone());
        assert!(
            canonical_result.is_err(),
            "DataContract::deserialize should run schema validation and reject \
             an invalid index. If this passes, validate-by-default has been \
             silently reverted."
        );

        // PIN: explicit opt-out accepts the same payload without validating.
        let unvalidated = DataContract::from_json(json.clone(), false, LATEST_PLATFORM_VERSION);
        assert!(
            unvalidated.is_ok(),
            "DataContract::from_json(_, false, _) should skip schema validation \
             and accept the structurally-well-formed payload."
        );

        // PIN: explicit validated path also rejects.
        let validated_result = DataContract::from_json(json, true, LATEST_PLATFORM_VERSION);
        assert!(
            validated_result.is_err(),
            "DataContract::from_json(_, true, _) should reject contracts with \
             invalid indices."
        );
    }
}
