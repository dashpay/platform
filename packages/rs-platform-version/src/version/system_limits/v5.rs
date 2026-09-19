use crate::version::system_limits::SystemLimits;

/// System limits for protocol version 15 and above. Relative to V4 this adds
/// the erase chunk: `max_document_revisions_erased_per_transition` bounds how
/// many retained revisions of a deleted keep-history document one erase
/// transition removes, so the work of erasing a long history is spread over
/// several transitions of bounded cost.
pub const SYSTEM_LIMITS_V5: SystemLimits = SystemLimits {
    max_document_revisions_erased_per_transition: Some(100),
    ..super::v4::SYSTEM_LIMITS_V4
};
