use dpp::identifier::Identifier;
use key_wallet::Network;

/// Errors that can occur in platform wallet operations
#[derive(Debug, thiserror::Error)]
pub enum PlatformWalletError {
    #[error("Wallet creation failed: {0}")]
    WalletCreation(String),

    #[error("Wallet not found: {0}")]
    WalletNotFound(String),

    #[error("Wallet already exists: {0}")]
    WalletAlreadyExists(String),

    #[error("Identity already exists: {0}")]
    IdentityAlreadyExists(Identifier),

    #[error("Identity not found: {0}")]
    IdentityNotFound(Identifier),

    #[error("No primary identity set")]
    NoPrimaryIdentity,

    #[error("Invalid identity data: {0}")]
    InvalidIdentityData(String),

    #[error("Contact request not found: {0}")]
    ContactRequestNotFound(Identifier),

    #[error("Identity index not set for identity {0} — register or discover the identity first")]
    IdentityIndexNotSet(Identifier),

    #[error(
        "DashPay receiving account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayReceivingAccountAlreadyExists {
        identity: Identifier,
        contact: Identifier,
        network: Network,
        account_index: u32,
    },

    #[error(
        "DashPay external account already exists for identity {identity} with contact {contact} on network {network:?} (account index {account_index})"
    )]
    DashpayExternalAccountAlreadyExists {
        identity: Identifier,
        contact: Identifier,
        network: Network,
        account_index: u32,
    },

    #[error("Asset lock transaction failed: {0}")]
    AssetLockTransaction(String),

    #[error("Transaction broadcast failed: {0}")]
    TransactionBroadcast(String),

    #[error("Transaction building failed: {0}")]
    TransactionBuild(String),

    #[error("Asset lock proof waiting failed: {0}")]
    AssetLockProofWait(String),

    #[error("SDK error: {0}")]
    Sdk(#[from] dash_sdk::Error),

    #[error("Address sync failed: {0}")]
    AddressSync(String),

    #[error("Address operation failed: {0}")]
    AddressOperation(String),

    #[error("Wallet is locked — unlock it before performing this operation")]
    WalletLocked,

    #[error("SPV is already running — stop it before starting again")]
    SpvAlreadyRunning,

    #[error("No wallets configured — add a wallet before starting SPV")]
    NoWalletsConfigured,

    #[error("SPV error: {0}")]
    SpvError(String),

    #[error("Token operation failed: {0}")]
    TokenError(String),

    #[error("Asset lock proof expired (IS proof too old, CL not yet available): {0}")]
    AssetLockExpired(String),

    #[error("Asset lock transaction not chain-locked, cannot fall back to CL proof: {0}")]
    AssetLockNotChainLocked(String),

    // --- Shielded pool errors (feature-gated) ---
    #[error("No unspent shielded notes available")]
    ShieldedNoUnspentNotes,

    #[error("Insufficient shielded balance: available {available}, required {required}")]
    ShieldedInsufficientBalance { available: u64, required: u64 },

    #[error("Shielded build error: {0}")]
    ShieldedBuildError(String),

    #[error("Shielded broadcast failed: {0}")]
    ShieldedBroadcastFailed(String),

    #[error("Shielded sync failed: {0}")]
    ShieldedSyncFailed(String),

    #[error("Shielded commitment tree update failed: {0}")]
    ShieldedTreeUpdateFailed(String),

    #[error("Shielded store error: {0}")]
    ShieldedStoreError(String),

    #[error("Shielded nullifier sync failed: {0}")]
    ShieldedNullifierSyncFailed(String),

    #[error("Shielded Merkle witness unavailable: {0}")]
    ShieldedMerkleWitnessUnavailable(String),

    #[error("Shielded key derivation failed: {0}")]
    ShieldedKeyDerivation(String),
}
