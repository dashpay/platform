use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub(super) fn find_duplicates_by_id<'a>(
    document_transitions: &'a Vec<&'a DocumentTransition>,
    platform_version: &PlatformVersion,
) -> Result<Vec<&'a DocumentTransition>, ProtocolError> {
    match platform_version
        .dpp
        .state_transitions
        .documents
        .documents_batch_transition
        .validation
        .find_duplicates_by_id
    {
        0 => Ok(v0::find_duplicates_by_id(document_transitions)),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "find_duplicates_by_id".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
