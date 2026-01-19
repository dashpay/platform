use dpp::identifier::Identifier;
use key_wallet::Network;

/// Errors that can occur in platform wallet operations
#[derive(Debug, thiserror::Error)]
pub enum PlatformWalletError {
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

    // Platform sync errors
    #[error("SDK not configured for platform sync")]
    SdkNotConfigured,

    #[error("No platform payment account found")]
    NoPlatformPaymentAccount,

    #[error("Platform sync error: {0}")]
    SyncError(String),

    #[error("Address derivation error: {0}")]
    AddressDerivationError(String),

    #[error("Invalid platform address: {0}")]
    InvalidPlatformAddress(String),
}
