//! Manual `Serialize` / `Deserialize` for the outer `DataContract` enum.
//!
//! # Critical-4: serde impurity (pinned by tests below)
//!
//! Both impls call `PlatformVersion::get_version_or_current_or_latest(None)`,
//! making serialization output *depend on a process-global thread-local-ish*
//! state — same DataContract value, different bytes if the active platform
//! version changes. This is by design: `DataContract` is a versioned enum
//! routed through `DataContractInSerializationFormat`, and the format depends
//! on the current platform.
//!
//! The Deserialize side additionally hardcodes `full_validation = true` —
//! every JSON / Value / CBOR deserialize path runs full schema validation,
//! regardless of whether the input was previously trusted. The hardcoded
//! comment explains why: "when deserializing from json/platform_value/cbor
//! we always want to validate (as this is not coming from the state)."
//!
//! **Why this is KEEP-AS-EXCEPTION**: the alternative (stateless serde) would
//! require burning the platform version into the wire shape itself, which we
//! already do via `DataContract::serialize_to_bytes_with_platform_version`
//! (the bincode storage path). The serde path is for human-readable surfaces
//! (JSON/CBOR/Value) where we accept the global-coupling trade-off. See
//! `docs/json-value-unification-plan.md` §3.0 Critical-4.
//!
//! The `data_contract_serde_pins_critical_4` test module below pins this
//! behavior so future refactors can't silently change it.

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
        // when deserializing from json/platform_value/cbor we always want to validate (as this is not coming from the state)
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
        let recovered: DataContract =
            serde_json::from_value(json).expect("deserialize from json");

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
        let format_json =
            serde_json::to_value(&format).expect("SerializationFormat -> json");

        assert_eq!(
            direct_json, format_json,
            "DataContract::serialize should be byte-equivalent to \
             DataContractInSerializationFormat::serialize at the current \
             platform version. If this fails, the manual serde impl has \
             diverged from the format-routing pattern documented in the \
             module-level comment."
        );
    }

    /// PIN: `DataContract::deserialize` enforces full schema validation —
    /// i.e., it hardcodes `full_validation = true` in its call to
    /// `try_from_platform_versioned`. We exercise this by feeding a
    /// well-formed `DataContractInSerializationFormat` whose document
    /// schema is structurally invalid (an `indices` entry referencing a
    /// nonexistent property). The format-level deserialize accepts it
    /// (no validation there); the `DataContract::deserialize` path must
    /// reject it (validation runs).
    #[test]
    fn data_contract_deserialize_rejects_invalid_schema_via_full_validation() {
        // Build a valid contract, then mutate its JSON to make the schema
        // semantically invalid: declare an index over a property not in
        // the schema's `properties` map. `DataContractInSerializationFormat`
        // has no validation hook for this; only `try_from_platform_versioned`
        // with `full_validation=true` catches it.
        let created = get_data_contract_fixture(None, 0, 1);
        let original = created.data_contract().clone();

        let mut json = serde_json::to_value(&original).expect("to_json");

        // Inject an index referencing a property the schema doesn't define.
        let document_schemas = json
            .get_mut("documentSchemas")
            .and_then(|v| v.as_object_mut())
            .expect("documentSchemas object");
        let (_, first_schema) = document_schemas
            .iter_mut()
            .next()
            .expect("at least one document schema");
        let schema_obj = first_schema
            .as_object_mut()
            .expect("schema is object");
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

        // Format-level deserialize succeeds (no validation):
        let format: DataContractInSerializationFormat =
            serde_json::from_value(json.clone()).expect(
                "format-level deserialize should accept structurally-valid input",
            );
        // Just to use the variable and prove the path runs:
        let _ = format;

        // DataContract-level deserialize must reject it (validation):
        let result: Result<DataContract, _> = serde_json::from_value(json);
        assert!(
            result.is_err(),
            "DataContract::deserialize should reject contracts with invalid \
             indices because Critical-4 hardcodes full_validation=true. If \
             this passes, the validation behavior has been silently disabled \
             and bypasses the documented invariant."
        );
    }
}
