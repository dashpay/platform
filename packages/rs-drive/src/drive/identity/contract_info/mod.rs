#[cfg(feature = "server")]
pub(crate) mod identity_contract_nonce;
#[cfg(feature = "server")]
pub mod keys;

/// The sub elements in the contract space for each identity.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ContractInfoStructure {
    /// The identity contract nonce to stop replay attacks
    IdentityContractNonceKey = 0,
    /// The contract bound keys
    ContractInfoKeysKey = 1,
}
