//! Contact request management for PlatformWalletInfo
//!
//! This module provides contact request functionality at the wallet level,
//! delegating to the appropriate ManagedIdentity.

use key_wallet::account::account_collection::DashpayAccountKey;
use super::PlatformWalletInfo;
use crate::{ContactRequest, EstablishedContact};
use crate::error::PlatformWalletError;
use dpp::prelude::Identifier;
use key_wallet::Network;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

impl PlatformWalletInfo {
    /// Add a sent contact request for a specific identity on a specific network
    /// If there's already an incoming request from the recipient, automatically establish the contact
    pub fn add_sent_contact_request(
        &mut self,
        account_index: u32,
        network: Network,
        identity_id: &Identifier,
        request: ContactRequest,
    ) -> Result<(), PlatformWalletError> {
        let managed_identity = self
            .identity_manager_mut(network)
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let friend_identity_id = request.recipient_id.to_buffer();

        managed_identity.add_sent_contact_request(request);

        let managed_account_collection = self.wallet_info.accounts_mut(network)
            .ok_or(PlatformWalletError::NoAccountsForNetwork(network))?;

        managed_account_collection.dashpay_receival_accounts.insert(DashpayAccountKey {
            index: account_index,
            user_identity_id: identity_id.to_buffer(),
            friend_identity_id,
        }, managed_account);
        Ok(())
    }

    /// Add an incoming contact request for a specific identity on a specific network
    /// If there's already a sent request to the sender, automatically establish the contact
    pub fn add_incoming_contact_request(
        &mut self,
        network: Network,
        identity_id: &Identifier,
        request: ContactRequest,
    ) -> Result<(), PlatformWalletError> {
        let managed_identity = self
            .identity_manager_mut(network)
            .managed_identity_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        managed_identity.add_incoming_contact_request(request);
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
