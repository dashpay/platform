use crate::error::PlatformWalletError;
use crate::ContactRequest;
use crate::IdentityManager;
use dpp::prelude::Identifier;
use key_wallet::wallet::ManagedWalletInfo;
use key_wallet::Network;
use std::fmt;

mod accessors;
mod contact_requests;
mod identity_discovery;
pub(crate) mod key_derivation;
mod managed_account_operations;
mod matured_transactions;
mod wallet_info_interface;
mod wallet_transaction_checker;

/// Platform wallet information that extends ManagedWalletInfo with identity support
#[derive(Clone)]
pub struct PlatformWalletInfo {
    /// The underlying managed wallet info
    pub wallet_info: ManagedWalletInfo,

    /// Identity manager
    pub identity_manager: IdentityManager,
}

impl PlatformWalletInfo {
    /// Create a new platform wallet info for a specific network
    pub fn new(network: Network, wallet_id: [u8; 32], name: String) -> Self {
        Self {
            wallet_info: ManagedWalletInfo::with_name(network, wallet_id, name),
            identity_manager: IdentityManager::new(),
        }
    }

    /// Get or create an identity manager
    fn identity_manager_mut(&mut self) -> &mut IdentityManager {
        &mut self.identity_manager
    }

    /// Get an identity manager (if it exists)
    fn identity_manager(&self) -> &IdentityManager {
        &self.identity_manager
    }
}

impl fmt::Debug for PlatformWalletInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlatformWalletInfo")
            .field("wallet_info", &self.wallet_info)
            .field("identity_manager", &self.identity_manager)
            .finish()
    }
}

/// Parse a contact request document into a ContactRequest struct
///
/// Extracts DashPay contact request fields from a platform document.
pub(super) fn parse_contact_request_document(
    doc: &dpp::document::Document,
) -> Result<ContactRequest, PlatformWalletError> {
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let properties = doc.properties();

    let to_user_id = properties
        .get("toUserId")
        .and_then(|v| match v {
            Value::Identifier(id) => Some(Identifier::from(*id)),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid toUserId in contact request".to_string(),
            )
        })?;

    let sender_key_index = properties
        .get("senderKeyIndex")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid senderKeyIndex in contact request".to_string(),
            )
        })?;

    let recipient_key_index = properties
        .get("recipientKeyIndex")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid recipientKeyIndex in contact request".to_string(),
            )
        })?;

    let account_reference = properties
        .get("accountReference")
        .and_then(|v| match v {
            Value::U32(i) => Some(*i),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid accountReference in contact request".to_string(),
            )
        })?;

    let encrypted_public_key = properties
        .get("encryptedPublicKey")
        .and_then(|v| match v {
            Value::Bytes(b) => Some(b.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(
                "Missing or invalid encryptedPublicKey in contact request".to_string(),
            )
        })?;

    let created_at_core_block_height = doc.created_at_core_block_height().unwrap_or(0);

    let created_at = doc.created_at().unwrap_or(0);

    let sender_id = doc.owner_id();

    Ok(ContactRequest::new(
        sender_id,
        to_user_id,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_public_key,
        created_at_core_block_height,
        created_at,
    ))
}

#[cfg(test)]
mod tests {
    use crate::platform_wallet_info::PlatformWalletInfo;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
    use key_wallet::Network;

    #[test]
    fn test_platform_wallet_creation() {
        let wallet_id = [1u8; 32];
        let wallet = PlatformWalletInfo::new(
            Network::Testnet,
            wallet_id,
            "Test Platform Wallet".to_string(),
        );

        assert_eq!(wallet.wallet_id(), wallet_id);
        assert_eq!(wallet.name(), Some("Test Platform Wallet"));
        assert_eq!(wallet.identities().len(), 0);
    }
}
