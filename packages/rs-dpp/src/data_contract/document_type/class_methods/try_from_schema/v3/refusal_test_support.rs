//! What the parser suites share: the refusal assertion of the index keyword
//! tests, and the value a whole contract is parsed from.

use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use platform_value::{platform_value, Value};
use std::collections::BTreeMap;

/// The id of the contract [`contract_value`] builds.
pub(super) const CONTRACT_ID: [u8; 32] = [7; 32];

/// A format 1 contract at `version` with `document_schemas`, and the contract's
/// `$defs` when `schema_defs` is given, owned by `[8; 32]`.
pub(super) fn contract_value(
    version: u32,
    document_schemas: Value,
    schema_defs: Option<BTreeMap<String, Value>>,
) -> Value {
    let mut contract = platform_value!({
        "$formatVersion": "1",
        "id": Value::Identifier(CONTRACT_ID),
        "ownerId": Value::Identifier([8; 32]),
        "version": version,
        "documentSchemas": document_schemas,
    });
    if let Some(schema_defs) = schema_defs {
        contract
            .set_value("schemaDefs", Value::from(schema_defs))
            .expect("the contract is a map");
    }
    contract
}

/// Asserts `result` refuses the contract with an error naming `fragment`, as
/// a consensus error: a paid refusal needs the consensus variant, since a bare
/// data contract error would surface as an internal error in a block.
pub(super) fn assert_refused(
    result: Result<BTreeMap<String, DocumentType>, ProtocolError>,
    fragment: &str,
) {
    let error = result.expect_err("the contract should be refused");
    assert!(
        matches!(error, ProtocolError::ConsensusError(_)),
        "expected a consensus error, got {error:?}"
    );
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
}
