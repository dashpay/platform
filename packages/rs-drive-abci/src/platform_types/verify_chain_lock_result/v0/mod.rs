/// A structure for returning the result of verification of a chain lock
pub struct VerifyChainLockResult {
    /// If the chain lock signature represents a valid G2 BLS Element
    pub(crate) chain_lock_signature_is_deserializable: bool,
    /// If the chain lock was found to be valid based on platform state
    pub(crate) found_valid_locally: Option<bool>,
    /// If we were not able to validate based on platform state - ie the chain lock is too far in
    /// the future
    pub(crate) found_valid_by_core: Option<bool>,
    /// Core is synced
    pub(crate) core_is_synced: Option<bool>,
}
