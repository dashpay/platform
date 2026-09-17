use crate::fee::Credits;
use crate::identity::TimestampMillis;

/// Trait for the getters added with `IdentityPublicKeyV1`. A V0 key answers `None` to both.
pub trait IdentityPublicKeyGettersV1 {
    /// The total credits that state transitions signed with this key may take from the
    /// identity, `None` when the key has no budget.
    fn budget(&self) -> Option<Credits>;

    /// The block time, in milliseconds, from which the key can no longer sign, `None` when the
    /// key does not expire.
    fn expires_at(&self) -> Option<TimestampMillis>;

    /// Has the key expired at the given block time. The expiry instant itself is already expired.
    fn is_expired_at(&self, time_ms: TimestampMillis) -> bool {
        self.expires_at()
            .is_some_and(|expires_at| time_ms >= expires_at)
    }

    /// Does the key carry a budget or an expiry
    fn has_limits(&self) -> bool {
        self.budget().is_some() || self.expires_at().is_some()
    }
}
