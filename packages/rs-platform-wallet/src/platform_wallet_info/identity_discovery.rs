//! Gap-limit identity discovery for wallet sync
//!
//! This module implements DashSync-style gap-limit scanning for identities
//! during wallet sync. It derives consecutive authentication keys from the
//! wallet's BIP32 tree and queries Platform to find registered identities.

use super::key_derivation::derive_identity_auth_key_hash;
use super::parse_contact_request_document;
use super::PlatformWalletInfo;
use crate::error::PlatformWalletError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

impl PlatformWalletInfo {
    /// Discover identities by scanning consecutive identity indices with a gap limit
    ///
    /// Starting from `start_index`, derives ECDSA authentication keys for consecutive
    /// identity indices and queries Platform for registered identities. Scanning stops
    /// when `gap_limit` consecutive indices yield no identity.
    ///
    /// This mirrors the DashSync gap-limit approach: keep scanning until N consecutive
    /// misses, then stop.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet to derive authentication keys from
    /// * `start_index` - The first identity index to check
    /// * `gap_limit` - Number of consecutive misses before stopping (typically 5)
    ///
    /// # Returns
    ///
    /// Returns the list of newly discovered identity IDs
    pub async fn discover_identities(
        &mut self,
        wallet: &key_wallet::Wallet,
        start_index: u32,
        gap_limit: u32,
    ) -> Result<Vec<Identifier>, PlatformWalletError> {
        use dash_sdk::platform::types::identity::PublicKeyHash;
        use dash_sdk::platform::Fetch;

        let sdk = self
            .identity_manager()
            .sdk
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "SDK not configured in identity manager".to_string(),
                )
            })?
            .clone();

        let network = self.network();
        let mut discovered = Vec::new();
        let mut consecutive_misses = 0u32;
        let mut identity_index = start_index;

        while consecutive_misses < gap_limit {
            // Derive the authentication key hash for this identity index (key_index 0)
            let key_hash_array =
                derive_identity_auth_key_hash(wallet, network, identity_index, 0)?;

            // Query Platform for an identity registered with this key hash
            match dpp::identity::Identity::fetch(&sdk, PublicKeyHash(key_hash_array)).await {
                Ok(Some(identity)) => {
                    let identity_id = identity.id();

                    // Add to manager if not already present
                    if !self
                        .identity_manager()
                        .identities()
                        .contains_key(&identity_id)
                    {
                        self.identity_manager_mut().add_identity(identity)?;
                    }

                    discovered.push(identity_id);
                    consecutive_misses = 0;
                }
                Ok(None) => {
                    consecutive_misses += 1;
                }
                Err(e) => {
                    eprintln!(
                        "Failed to query identity by public key hash at index {}: {}",
                        identity_index, e
                    );
                    consecutive_misses += 1;
                }
            }

            identity_index += 1;
        }

        Ok(discovered)
    }

    /// Discover identities and fetch their DashPay contact requests
    ///
    /// Calls [`discover_identities`] then fetches sent and received contact requests
    /// for each discovered identity, storing them in the identity manager.
    ///
    /// # Arguments
    ///
    /// * `wallet` - The wallet to derive authentication keys from
    /// * `start_index` - The first identity index to check
    /// * `gap_limit` - Number of consecutive misses before stopping (typically 5)
    ///
    /// # Returns
    ///
    /// Returns the list of newly discovered identity IDs
    pub async fn discover_identities_with_contacts(
        &mut self,
        wallet: &key_wallet::Wallet,
        start_index: u32,
        gap_limit: u32,
    ) -> Result<Vec<Identifier>, PlatformWalletError> {
        let discovered = self
            .discover_identities(wallet, start_index, gap_limit)
            .await?;

        if discovered.is_empty() {
            return Ok(discovered);
        }

        let sdk = self
            .identity_manager()
            .sdk
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "SDK not configured in identity manager".to_string(),
                )
            })?
            .clone();

        for identity_id in &discovered {
            // Get the identity from the manager to pass to the SDK
            let identity = match self.identity_manager().identity(identity_id) {
                Some(id) => id.clone(),
                None => continue,
            };

            match sdk
                .fetch_all_contact_requests_for_identity(&identity, Some(100))
                .await
            {
                Ok((sent_docs, received_docs)) => {
                    // Process sent contact requests
                    for (_doc_id, maybe_doc) in sent_docs {
                        if let Some(doc) = maybe_doc {
                            if let Ok(contact_request) = parse_contact_request_document(&doc) {
                                if let Some(managed_identity) = self
                                    .identity_manager_mut()
                                    .managed_identity_mut(identity_id)
                                {
                                    managed_identity.add_sent_contact_request(contact_request);
                                }
                            }
                        }
                    }

                    // Process received contact requests
                    for (_doc_id, maybe_doc) in received_docs {
                        if let Some(doc) = maybe_doc {
                            if let Ok(contact_request) = parse_contact_request_document(&doc) {
                                if let Some(managed_identity) = self
                                    .identity_manager_mut()
                                    .managed_identity_mut(identity_id)
                                {
                                    managed_identity.add_incoming_contact_request(contact_request);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to fetch contact requests for identity {}: {}",
                        identity_id, e
                    );
                }
            }
        }

        Ok(discovered)
    }
}
