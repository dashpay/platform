use super::PrimaryKeyPathQueryTarget;

/// Primary-key layout introduced in protocol version 15.
pub(super) const fn primary_key_path_query_target_v1() -> PrimaryKeyPathQueryTarget {
    PrimaryKeyPathQueryTarget::CurrentDocument
}
