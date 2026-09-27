//! Test helpers the typed array suites share: a parse through the real
//! `try_from_schema` dispatcher and the two error shapes a refused parse
//! comes back as.

use super::*;
use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::errors::DataContractError;

/// Parse through the real dispatcher, which picks the parser generation out
/// of the platform version's `try_from_schema` table value (generation 2 at
/// PV13, generation 3 at PV14).
pub(super) fn parse_dispatched(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "charter",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// The error of a validating parse the meta-schema refused.
pub(super) fn expect_json_schema_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
) -> JsonSchemaError {
    match result {
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::JsonSchemaError(error)) => error,
            other => panic!("expected a JSON schema error, got {other:?}"),
        },
        other => panic!("expected a JSON schema error, got {other:?}"),
    }
}

/// The parser's structure errors surface as `InvalidContractStructure`
/// either directly or, with the `validation` feature on, wrapped as the basic
/// `ContractError`.
pub(super) fn expect_structure_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
    needle: &str,
) {
    let message = match result {
        Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            message,
        ))) => message,
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message),
            )) => message,
            other => panic!("expected InvalidContractStructure, got {other:?}"),
        },
        other => panic!("expected InvalidContractStructure, got {other:?}"),
    };
    assert!(
        message.contains(needle),
        "expected {needle:?} in the error, got: {message}"
    );
}
