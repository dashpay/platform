use crate::fee::Credits;
use crate::identity::TimestampMillis;
use crate::ProtocolError;

/// Trait for the getters added with `IdentityPublicKeyV1`. A V0 key answers `None` to both.
pub trait IdentityPublicKeyGettersV1 {
    /// The total credits that state transitions signed with this key may take from the
    /// identity over its lifetime, `None` when the key has no budget. What is left of it is not
    /// in the key: Drive tracks it.
    fn total_budget(&self) -> Option<Credits>;

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
        self.total_budget().is_some() || self.expires_at().is_some()
    }
}

/// Trait for the setters added with `IdentityPublicKeyV1`. A V0 key has no limits to set and
/// answers with an error; the identity key limits update transition only ever reaches V1 keys.
pub trait IdentityPublicKeySettersV1 {
    /// Sets the total budget of the key
    fn set_total_budget(&mut self, total_budget: Option<Credits>) -> Result<(), ProtocolError>;

    /// Sets the expiry of the key
    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>) -> Result<(), ProtocolError>;
}
