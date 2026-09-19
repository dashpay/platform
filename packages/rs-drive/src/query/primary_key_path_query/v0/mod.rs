use super::PrimaryKeyPathQueryTarget;

/// Primary-key layout used through protocol version 14.
pub(super) const fn primary_key_path_query_target_v0() -> PrimaryKeyPathQueryTarget {
    PrimaryKeyPathQueryTarget::LegacyHistoryTree
}
