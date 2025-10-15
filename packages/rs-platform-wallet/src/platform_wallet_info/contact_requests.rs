//! Contact request management for PlatformWalletInfo
//!
//! This module provides contact request functionality at the wallet level,
//! delegating to the appropriate ManagedIdentity.

use super::PlatformWalletInfo;
use crate::error::PlatformWalletError;
use crate::{ContactRequest, EstablishedContact};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::account::account_collection::DashpayAccountKey;
use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedAccountOperations;
use key_wallet::Network;
use key_wallet::Wallet;

impl PlatformWalletInfo {
    /// Add a sent contact request for a specific identity on a specific network
    /// If there's already an incoming request from the recipient, automatically establish the contact
    pub fn add_sent_contact_request(
        &mut self,
        wallet: &mut Wallet,
        account_index: u32,
        network: Network,
        identity_id: &Identifier,
        request: ContactRequest,
    ) -> Result<(), PlatformWalletError> {
        if self
            .identity_manager(network)
            .and_then(|manager| manager.managed_identity(identity_id))
            .is_none()
        {
            return Err(PlatformWalletError::IdentityNotFound(*identity_id));
        }

        let friend_identity_id = request.recipient_id.to_buffer();
        let request_created_at = request.created_at;
        let user_identity_id = identity_id.to_buffer();

        let account_key = DashpayAccountKey {
            index: account_index,
            user_identity_id,
            friend_identity_id,
        };

        let account_type = AccountType::DashpayReceivingFunds {
            index: account_index,
            user_identity_id,
            friend_identity_id,
        };

        let wallet_has_account = wallet
            .accounts
            .get(&network)
            .and_then(|collection| collection.account_of_type(account_type))
            .is_some();

        if wallet_has_account {
            return Err(PlatformWalletError::DashpayReceivingAccountAlreadyExists {
                identity: *identity_id,
                contact: Identifier::from(friend_identity_id),
                network,
                account_index,
            });
        }

        if !wallet_has_account {
            let account_path = account_type.derivation_path(network).map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to derive DashPay receiving account path: {err}"
                ))
            })?;

            let account_xpub = wallet
                .derive_extended_public_key(network, &account_path)
                .map_err(|err| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to derive DashPay receiving account xpub: {err}"
                    ))
                })?;

            wallet
                .add_account(account_type, network, Some(account_xpub))
                .map_err(|err| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to add DashPay receiving account to wallet: {err}"
                    ))
                })?;
        }

        let managed_has_account = self
            .wallet_info
            .accounts(network)
            .and_then(|collection| collection.dashpay_receival_accounts.get(&account_key))
            .is_some();

        if managed_has_account {
            return Err(PlatformWalletError::DashpayReceivingAccountAlreadyExists {
                identity: *identity_id,
                contact: Identifier::from(friend_identity_id),
                network,
                account_index,
            });
        }

        if !managed_has_account {
            self.wallet_info
                .add_managed_account(wallet, account_type, network)
                .map_err(|err| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "Failed to add managed DashPay receiving account: {err}"
                    ))
                })?;
        }

        let managed_account_collection = self
            .wallet_info
            .accounts_mut(network)
            .ok_or(PlatformWalletError::NoAccountsForNetwork(network))?;

        let managed_account = managed_account_collection
            .dashpay_receival_accounts
            .get_mut(&account_key)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Managed DashPay receiving account is missing".to_string(),
                )
            })?;

        managed_account.metadata.last_used = Some(request_created_at);

        self.identity_manager_mut(network)
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
            .add_sent_contact_request(request);

        Ok(())
    }

    /// Add an incoming contact request for a specific identity on a specific network
    /// If there's already a sent request to the sender, automatically establish the contact
    pub fn add_incoming_contact_request(
        &mut self,
        wallet: &mut Wallet,
        network: Network,
        identity_id: &Identifier,
        friend_identity: &Identity,
        request: ContactRequest,
    ) -> Result<(), PlatformWalletError> {
        if self
            .identity_manager(network)
            .and_then(|manager| manager.managed_identity(identity_id))
            .is_none()
        {
            return Err(PlatformWalletError::IdentityNotFound(*identity_id));
        }

        if friend_identity.id() != request.sender_id {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Incoming contact request sender does not match provided identity".to_string(),
            ));
        }

        let sender_key = friend_identity
            .public_keys()
            .get(&request.sender_key_index)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Sender identity is missing the declared encryption key".to_string(),
                )
            })?;

        if sender_key.purpose() != Purpose::ENCRYPTION {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Sender key purpose must be ENCRYPTION".to_string(),
            ));
        }

        if self
            .identity_manager(network)
            .and_then(|manager| manager.managed_identity(identity_id))
            .and_then(|managed| {
                managed
                    .identity
                    .public_keys()
                    .get(&request.recipient_key_index)
            })
            .is_none()
        {
            return Err(PlatformWalletError::InvalidIdentityData(
                "Recipient identity is missing the declared encryption key".to_string(),
            ));
        }

        let request_created_at = request.created_at;
        let friend_identity_id = request.sender_id.to_buffer();
        let friend_identity_identifier = Identifier::from(friend_identity_id);
        let user_identity_id = identity_id.to_buffer();
        let account_index = request.account_reference;
        let encrypted_public_key = request.encrypted_public_key.clone();

        let account_key = DashpayAccountKey {
            index: account_index,
            user_identity_id,
            friend_identity_id,
        };

        let account_type = AccountType::DashpayExternalAccount {
            index: account_index,
            user_identity_id,
            friend_identity_id,
        };

        let wallet_has_account = wallet
            .accounts
            .get(&network)
            .and_then(|collection| collection.account_of_type(account_type))
            .is_some();

        if wallet_has_account {
            return Err(PlatformWalletError::DashpayExternalAccountAlreadyExists {
                identity: *identity_id,
                contact: friend_identity_identifier,
                network,
                account_index,
            });
        }

        let account_xpub = ExtendedPubKey::decode(&encrypted_public_key).map_err(|err| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Failed to decode DashPay contact account xpub: {err}"
            ))
        })?;

        wallet
            .add_account(account_type, network, Some(account_xpub))
            .map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to add DashPay external account to wallet: {err}"
                ))
            })?;

        let managed_has_account = self
            .wallet_info
            .accounts(network)
            .and_then(|collection| collection.dashpay_external_accounts.get(&account_key))
            .is_some();

        if managed_has_account {
            return Err(PlatformWalletError::DashpayExternalAccountAlreadyExists {
                identity: *identity_id,
                contact: friend_identity_identifier,
                network,
                account_index,
            });
        }

        self.wallet_info
            .add_managed_account(wallet, account_type, network)
            .map_err(|err| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to add managed DashPay external account: {err}"
                ))
            })?;

        let managed_account_collection = self
            .wallet_info
            .accounts_mut(network)
            .ok_or(PlatformWalletError::NoAccountsForNetwork(network))?;

        let managed_account = managed_account_collection
            .dashpay_external_accounts
            .get_mut(&account_key)
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "Managed DashPay external account is missing".to_string(),
                )
            })?;

        managed_account.metadata.last_used = Some(request_created_at);

        self.identity_manager_mut(network)
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
            .add_incoming_contact_request(request);

        Ok(())
    }

    /// Accept an incoming contact request and establish the contact on a specific network
    /// Returns the established contact if successful
    pub fn accept_incoming_request(
        &mut self,
        network: Network,
        identity_id: &Identifier,
        sender_id: &Identifier,
    ) -> Result<EstablishedContact, PlatformWalletError> {
        let managed_identity = self
            .identity_manager_mut(network)
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        managed_identity
            .accept_incoming_request(sender_id)
            .ok_or(PlatformWalletError::ContactRequestNotFound(*sender_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_wallet_info::PlatformWalletInfo;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::identity_public_key::IdentityPublicKey;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::bip32::ExtendedPubKey;
    use key_wallet::Network;
    use std::collections::BTreeMap;

    fn create_dummy_wallet() -> Wallet {
        // Create a dummy extended public key for testing
        use key_wallet::wallet::root_extended_keys::RootExtendedPubKey;
        let xpub_str = "xpub6ASuArnXKPbfEwhqN6e3mwBcDTgzisQN1wXN9BJcM47sSikHjJf3UFHKkNAWbWMiGj7Wf5uMash7SyYq527Hqck2AxYysAA7xmALppuCkwQ";
        let xpub = xpub_str.parse::<ExtendedPubKey>().unwrap();
        let root_xpub = RootExtendedPubKey::from_extended_pub_key(&xpub);
        Wallet::from_wallet_type(key_wallet::wallet::WalletType::WatchOnly(root_xpub))
    }

    fn create_test_identity(id_bytes: [u8; 32]) -> Identity {
        let mut public_keys = BTreeMap::new();

        // Add encryption key at index 0
        let encryption_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::ENCRYPTION,
            security_level: dpp::identity::SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: dpp::identity::KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![1u8; 33]),
            disabled_at: None,
        });

        public_keys.insert(0, encryption_key);

        let identity_v0 = IdentityV0 {
            id: Identifier::from(id_bytes),
            public_keys,
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    fn create_contact_request(
        sender_id: Identifier,
        recipient_id: Identifier,
        timestamp: u64,
    ) -> ContactRequest {
        ContactRequest::new(
            sender_id,
            recipient_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            timestamp,
        )
    }

    #[test]
    fn test_accept_incoming_request_identity_not_found() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);

        // Try to accept request for non-existent identity
        let result = platform_wallet.accept_incoming_request(network, &identity_id, &sender_id);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::IdentityNotFound(_)
        ));
    }

    #[test]
    fn test_accept_incoming_request_contact_not_found() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);

        // Create and add identity
        let identity = create_test_identity([1u8; 32]);
        platform_wallet
            .identity_manager_mut(network)
            .add_identity(identity)
            .unwrap();

        // Try to accept request that doesn't exist
        let result = platform_wallet.accept_incoming_request(network, &identity_id, &sender_id);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::ContactRequestNotFound(_)
        ));
    }

    #[test]
    fn test_error_identity_not_found_for_sent_request() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let recipient_id = Identifier::from([2u8; 32]);

        let request = create_contact_request(identity_id, recipient_id, 1234567890);

        // Try to add sent request for non-existent identity
        let result = platform_wallet.add_sent_contact_request(
            &mut wallet,
            0,
            network,
            &identity_id,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::IdentityNotFound(_)
        ));
    }

    #[test]
    fn test_error_identity_not_found_for_incoming_request() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let friend_id = Identifier::from([2u8; 32]);

        let friend_identity = create_test_identity([2u8; 32]);
        let request = create_contact_request(friend_id, identity_id, 1234567890);

        // Try to add incoming request for non-existent identity
        let result = platform_wallet.add_incoming_contact_request(
            &mut wallet,
            network,
            &identity_id,
            &friend_identity,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::IdentityNotFound(_)
        ));
    }

    #[test]
    fn test_error_sender_mismatch_for_incoming_request() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let friend_id = Identifier::from([2u8; 32]);
        let wrong_id = Identifier::from([3u8; 32]);

        // Create and add our identity
        let identity = create_test_identity([1u8; 32]);
        platform_wallet
            .identity_manager_mut(network)
            .add_identity(identity)
            .unwrap();

        // Create friend identity with one ID
        let friend_identity = create_test_identity([2u8; 32]);

        // Create request with wrong sender ID
        let request = create_contact_request(wrong_id, identity_id, 1234567890);

        // Try to add incoming request with mismatched sender
        let result = platform_wallet.add_incoming_contact_request(
            &mut wallet,
            network,
            &identity_id,
            &friend_identity,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::InvalidIdentityData(_)
        ));
    }

    #[test]
    fn test_error_missing_encryption_key_in_sender() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let friend_id = Identifier::from([2u8; 32]);

        // Create and add our identity
        let identity = create_test_identity([1u8; 32]);
        platform_wallet
            .identity_manager_mut(network)
            .add_identity(identity)
            .unwrap();

        // Create friend identity without encryption key
        let identity_v0 = IdentityV0 {
            id: friend_id,
            public_keys: BTreeMap::new(), // Empty - no encryption key
            balance: 1000,
            revision: 1,
        };
        let friend_identity = Identity::V0(identity_v0);

        // Create request referencing non-existent key
        let mut request = create_contact_request(friend_id, identity_id, 1234567890);
        request.sender_key_index = 99; // Reference non-existent key

        // Try to add incoming request
        let result = platform_wallet.add_incoming_contact_request(
            &mut wallet,
            network,
            &identity_id,
            &friend_identity,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::InvalidIdentityData(_)
        ));
    }

    #[test]
    fn test_error_wrong_key_purpose_in_sender() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let friend_id = Identifier::from([2u8; 32]);

        // Create and add our identity
        let identity = create_test_identity([1u8; 32]);
        platform_wallet
            .identity_manager_mut(network)
            .add_identity(identity)
            .unwrap();

        // Create friend identity with AUTHENTICATION key instead of ENCRYPTION
        let mut public_keys = BTreeMap::new();
        let auth_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION, // Wrong purpose
            security_level: dpp::identity::SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: dpp::identity::KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![1u8; 33]),
            disabled_at: None,
        });
        public_keys.insert(0, auth_key);

        let identity_v0 = IdentityV0 {
            id: friend_id,
            public_keys,
            balance: 1000,
            revision: 1,
        };
        let friend_identity = Identity::V0(identity_v0);

        let request = create_contact_request(friend_id, identity_id, 1234567890);

        // Try to add incoming request
        let result = platform_wallet.add_incoming_contact_request(
            &mut wallet,
            network,
            &identity_id,
            &friend_identity,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::InvalidIdentityData(_)
        ));
    }

    #[test]
    fn test_error_missing_recipient_encryption_key() {
        let mut platform_wallet = PlatformWalletInfo::new([1u8; 32], "Test Wallet".to_string());
        let mut wallet = create_dummy_wallet();
        let network = Network::Testnet;
        let identity_id = Identifier::from([1u8; 32]);
        let friend_id = Identifier::from([2u8; 32]);

        // Create and add our identity WITHOUT encryption key
        let identity_v0 = IdentityV0 {
            id: identity_id,
            public_keys: BTreeMap::new(), // No encryption key
            balance: 1000,
            revision: 1,
        };
        let identity = Identity::V0(identity_v0);
        platform_wallet
            .identity_manager_mut(network)
            .add_identity(identity)
            .unwrap();

        let friend_identity = create_test_identity([2u8; 32]);
        let mut request = create_contact_request(friend_id, identity_id, 1234567890);
        request.recipient_key_index = 99; // Reference non-existent key

        // Try to add incoming request
        let result = platform_wallet.add_incoming_contact_request(
            &mut wallet,
            network,
            &identity_id,
            &friend_identity,
            request,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PlatformWalletError::InvalidIdentityData(_)
        ));
    }
}
