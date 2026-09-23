//! Test helpers the `refersTo` parser suites share: the charter fixture's
//! `joinRequest` document type (permanent and immutable, unique on
//! (`submittedCharterId`, `$ownerId`), with a non-unique `byMessage` index),
//! identifier properties, a contract parse at a given protocol version, and
//! the refusal check.

use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::json;

/// The id the fixture contracts carry.
pub(super) const CONTRACT_ID: [u8; 32] = [7; 32];

/// An identifier property at `position`.
pub(super) fn identifier(position: u32) -> serde_json::Value {
    json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

/// The `joinRequest` document type the charter fixtures refer to.
pub(super) fn join_request_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "canBeDeleted": false,
        "documentsMutable": false,
        "properties": {
            "submittedCharterId": identifier(0),
            "message": { "type": "string", "maxLength": 63, "position": 1 }
        },
        "indices": [
            {
                "name": "bySubmittedCharter",
                "properties": [{ "submittedCharterId": "asc" }, { "$ownerId": "asc" }],
                "unique": true
            },
            { "name": "byMessage", "properties": [{ "message": "asc" }] }
        ],
        "required": ["submittedCharterId", "message"],
        "additionalProperties": false
    })
}

/// Parses `contract` at `platform_version`, registering (`full_validation`)
/// or reading back a stored contract.
pub(super) fn contract_on(
    contract: serde_json::Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = platform_value::to_value(contract).expect("the contract should convert");
    DataContract::from_value(value, full_validation, platform_version)
}

/// Registers `contract` at the latest protocol version.
pub(super) fn contract(contract: serde_json::Value) -> Result<DataContract, ProtocolError> {
    contract_on(contract, true, PlatformVersion::latest())
}

/// The contract was refused with `fragment` in the error.
pub(super) fn assert_refused(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract should be refused");
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
}
