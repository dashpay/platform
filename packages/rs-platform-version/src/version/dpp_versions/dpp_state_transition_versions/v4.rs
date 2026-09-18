use crate::version::dpp_versions::dpp_state_transition_versions::v3::STATE_TRANSITION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_versions::{
    DPPStateTransitionVersions, DocumentTransitionVersions,
    DocumentsBatchTransitionValidationVersions, DocumentsBatchTransitionVersions,
};

/// Protocol 15 selects the batch validator that sees every batch wire format,
/// including format 2, whose document shell carries the erase kind. The
/// generation selected by earlier protocol versions only sees the shell of
/// formats 0 and 1 and remains unchanged.
pub const STATE_TRANSITION_VERSIONS_V4: DPPStateTransitionVersions = DPPStateTransitionVersions {
    documents: DocumentTransitionVersions {
        documents_batch_transition: DocumentsBatchTransitionVersions {
            validation: DocumentsBatchTransitionValidationVersions {
                find_duplicates_by_id: 0,
                validate_base_structure: 1,
            },
        },
    },
    ..STATE_TRANSITION_VERSIONS_V3
};
