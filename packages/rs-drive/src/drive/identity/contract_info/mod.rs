#[cfg(feature = "full")]
pub mod keys;
#[cfg(feature = "full")]
pub(crate) mod revision_nonce;

/// The sub elements in the contract space for each identity.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ContractInfoStructure {
    /// The identity contract nonce to stop replay attacks
    IdentityContractNonceKey = 0,
    /// The contract bound keys
    ContractInfoKeysKey = 1,
}
