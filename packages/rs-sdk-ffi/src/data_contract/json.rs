//! The one JSON rendering of a data contract the FFI hands to Swift and
//! Kotlin.
//!
//! Every emitter (`fetch_json`, `fetch_many`, `fetch_by_range`,
//! `fetch_with_serialization`) renders through [`contract_json_value`] at
//! the SDK's network protocol version, so the JSON a mobile host persists
//! is the same envelope for every query and matches the proof-verified
//! fetch. The canonical `serde_json::to_value(&DataContract)` path instead
//! reads the process-global current platform version, which the SDK never
//! sets, so it would silently resolve to `PlatformVersion::latest()` and
//! diverge from the network the moment a newer serialization generation
//! exists.

use crate::error::{DashSDKError, DashSDKErrorCode};
use dash_sdk::dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dash_sdk::dpp::version::{PlatformVersion, TryIntoPlatformVersioned};
use dash_sdk::platform::DataContract;

/// The contract in its serialization format at `platform_version`, as JSON.
///
/// The envelope generation (`$formatVersion`) is chosen from the supplied
/// version's tables, and the nested `config` block keeps the generation the
/// contract carries; a pre-v11 contract loaded from state still renders a
/// V0 configuration inside a V1 envelope.
pub(crate) fn contract_json_value(
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Result<serde_json::Value, DashSDKError> {
    let format: DataContractInSerializationFormat = contract
        .try_into_platform_versioned(platform_version)
        .map_err(|e| {
            DashSDKError::new(
                DashSDKErrorCode::SerializationError,
                format!(
                    "Failed to convert contract to its serialization format: {}",
                    e
                ),
            )
        })?;

    serde_json::to_value(&format).map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to convert contract to JSON: {}", e),
        )
    })
}

#[cfg(test)]
mod tests {
    //! Replays the canonical contract configuration corpus pinned by rs-dpp
    //! (`packages/rs-dpp/src/data_contract/config/vectors`): the JSON this
    //! crate emits must be the canonical envelope at the supplied version.

    use super::*;
    use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use serde::Deserialize;

    const CORPUS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../rs-dpp/src/data_contract/config/vectors/contract_config_vectors.json"
    ));

    /// A protocol version whose tables select the V1 contract envelope.
    const V1_ENVELOPE_PROTOCOL_VERSION: u32 = 14;

    #[derive(Deserialize)]
    struct Corpus {
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        platform_version: u32,
        contract: serde_json::Value,
        canonical: serde_json::Value,
    }

    fn corpus() -> Corpus {
        serde_json::from_str(CORPUS).expect("the pinned corpus parses")
    }

    fn platform_version(protocol_version: u32) -> &'static PlatformVersion {
        PlatformVersion::get(protocol_version)
            .unwrap_or_else(|e| panic!("protocol version {protocol_version} is unknown: {e}"))
    }

    fn decode(case: &Case) -> DataContract {
        DataContract::from_json(
            case.contract.clone(),
            true,
            platform_version(case.platform_version),
        )
        .unwrap_or_else(|e| panic!("{}: the vector does not decode: {e}", case.name))
    }

    /// `DashSDKError` carries a raw C string and has no `Debug`; unwrap by
    /// the error code instead.
    fn render(contract: &DataContract, protocol_version: u32) -> serde_json::Value {
        contract_json_value(contract, platform_version(protocol_version))
            .unwrap_or_else(|e| panic!("rendering failed with code {:?}", e.code))
    }

    #[test]
    fn should_render_every_vector_as_the_canonical_envelope() {
        for case in corpus().cases {
            let contract = decode(&case);
            assert_eq!(
                render(&contract, case.platform_version),
                case.canonical,
                "{}: the FFI JSON differs from the canonical envelope",
                case.name
            );
        }
    }

    #[test]
    fn should_render_the_supplied_version_not_the_global_one() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.name == "v0_envelope")
            .expect("the corpus carries the historic V0 envelope case");
        let contract = decode(case);

        let historic = render(&contract, case.platform_version);
        assert_eq!(historic["$formatVersion"], "0");
        assert_eq!(historic["config"]["$formatVersion"], "0");

        // Rendered at a version whose tables select the V1 envelope, the
        // same contract comes back in the current envelope with its
        // configuration generation untouched. The argument alone decides.
        let current = render(&contract, V1_ENVELOPE_PROTOCOL_VERSION);
        assert_eq!(current["$formatVersion"], "1");
        assert_eq!(current["config"]["$formatVersion"], "0");
        assert_eq!(current["config"], historic["config"]);
    }
}
