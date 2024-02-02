pub mod keys;
mod revision_nonce;

/// The sub elements in the contract space for each identity.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ContractInfoStructure {
    /// The identity contract nonce to stop replay attacks
    IdentityContractNonce = 0,
    /// The contract bound keys
    ContractInfoKeys = 1,
}
