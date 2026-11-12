//! Where a context provider takes quorum keys from, and the modes of one that
//! can change it while the SDK runs.

/// Where an [`AdaptiveContextProvider`](crate::AdaptiveContextProvider) takes
/// quorum keys from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ContextProviderMode {
    /// SPV while its source is published and synced, trusted whenever it is
    /// not. The fallback can be forced, see
    /// [`AdaptiveContextProvider`](crate::AdaptiveContextProvider).
    Auto = 0,
    /// SPV only; lookups fail whenever its source is not published and
    /// synced, and never fall back to the trusted provider.
    Spv = 1,
    /// The trusted provider only.
    Trusted = 2,
}

impl TryFrom<u8> for ContextProviderMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Spv),
            2 => Ok(Self::Trusted),
            _ => Err(()),
        }
    }
}

/// The source a provider answers quorum lookups from, see
/// [`ContextProvider::quorum_source`](crate::ContextProvider::quorum_source).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum QuorumKeySource {
    /// A trusted quorum service.
    Trusted = 0,
    /// An SPV client.
    Spv = 1,
}
