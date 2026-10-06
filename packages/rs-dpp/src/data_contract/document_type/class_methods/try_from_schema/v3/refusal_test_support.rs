//! The refusal assertion the parser tests of index keywords share.

use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use std::collections::BTreeMap;

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
