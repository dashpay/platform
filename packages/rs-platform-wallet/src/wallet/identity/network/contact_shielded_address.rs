//! The shielded payment address a contact request carries.

use super::identity_handle::IdentityWallet;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
#[cfg(feature = "shielded")]
use crate::wallet::shielded::shielded_identity_account_index;
#[cfg(feature = "shielded")]
use dash_sdk::platform::Fetch;
#[cfg(feature = "shielded")]
use dpp::data_contract::accessors::v0::DataContractV0Getters;
#[cfg(feature = "shielded")]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "shielded")]
use dpp::data_contracts::SystemDataContract;
#[cfg(feature = "shielded")]
use dpp::prelude::DataContract;
use dpp::prelude::Identifier;

/// The DashPay document type of a contact request.
#[cfg(feature = "shielded")]
const CONTACT_REQUEST: &str = "contactRequest";

/// The `contactRequest` property that carries the sender's shielded address.
#[cfg(feature = "shielded")]
const ENCRYPTED_SHIELDED_ADDRESS: &str = "encryptedShieldedAddress";

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// The shielded payment address `owner_id` gives `to_user_id` in the contact request
    /// it sends under `account_reference`, or `None` when the request goes without one.
    ///
    /// The address is the one the identity's shielded account derives for this request
    /// alone (`AccountViewingKeys::contact_address`), so every contact gets a different
    /// one and a payment that arrives on it is attributed to that contact. It is `None`
    /// unless all of these hold:
    ///
    /// - the identity's shielded account was prepared in this session, which proves with
    ///   the seed that the identity holds the slot the account belongs to, and it is
    ///   still the account of the slot the identity holds;
    /// - that account's viewing keys are bound;
    /// - the connected network's contact requests have the property that carries the
    ///   address. A bundled schema does not show the network has activated it.
    ///
    /// The send is refused when the address would be one this identity has given
    /// another contact. That takes a collision in 87 bits of a keyed hash.
    ///
    /// The caller holds the identity's placement from this call until the request is
    /// signed, so the account is still the identity's when the request commits to it.
    pub(crate) async fn contact_shielded_address(
        &self,
        owner_id: &Identifier,
        to_user_id: &Identifier,
        account_reference: u32,
    ) -> Result<Option<[u8; 43]>, PlatformWalletError> {
        #[cfg(test)]
        if let Some(address) = test_hooks::given() {
            return Ok(Some(address));
        }
        #[cfg(not(feature = "shielded"))]
        {
            let _ = (owner_id, to_user_id, account_reference);
            Ok(None)
        }
        #[cfg(feature = "shielded")]
        {
            // The account of the slot the identity holds now, and the requests it has
            // sent to its other contacts.
            let (account, others) = {
                let wm = self.wallet_manager.read().await;
                let Some(managed) = wm
                    .get_wallet_info(&self.wallet_id)
                    .and_then(|info| info.identity_manager.managed_identity(owner_id))
                    .filter(|identity| identity.wallet_id == Some(self.wallet_id))
                else {
                    return Ok(None);
                };
                let Some(account) = managed
                    .identity_index
                    .and_then(|index| shielded_identity_account_index(index).ok())
                else {
                    return Ok(None);
                };
                let others: Vec<(Identifier, u32)> = managed
                    .dashpay()
                    .sent_contact_requests()
                    .values()
                    .filter(|request| request.recipient_id != *to_user_id)
                    .map(|request| (request.recipient_id, request.account_reference))
                    .collect();
                (account, others)
            };
            let Some(keys) = self
                .contact_shielded_addresses
                .proven_account_keys(owner_id, account)
                .await
            else {
                return Ok(None);
            };
            if !self.contact_requests_carry_shielded_address().await? {
                return Ok(None);
            }

            let owner = owner_id.as_bytes();
            let reference = u64::from(account_reference);
            let index = keys.contact_index(owner, to_user_id.as_bytes(), reference);
            let taken = others.iter().any(|(other, other_reference)| {
                keys.contact_index(owner, other.as_bytes(), u64::from(*other_reference)) == index
            });
            if taken {
                return Err(PlatformWalletError::InvalidIdentityData(
                    "The shielded address of this contact request is one already given to \
                     another contact; send the request under another account reference"
                        .to_string(),
                ));
            }
            Ok(Some(
                keys.contact_address(owner, to_user_id.as_bytes(), reference)
                    .to_raw_address_bytes(),
            ))
        }
    }

    /// Whether the connected network's DashPay contract gives contact requests the
    /// property that carries a shielded address, which it does from DashPay v3 at
    /// protocol version 15. Read from the network: the bundled schema is the latest one.
    #[cfg(feature = "shielded")]
    async fn contact_requests_carry_shielded_address(&self) -> Result<bool, PlatformWalletError> {
        let contract = DataContract::fetch(&self.sdk, SystemDataContract::Dashpay.id())
            .await?
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "DashPay contract is not available".to_string(),
                )
            })?;
        let contact_request = contract
            .document_type_for_name(CONTACT_REQUEST)
            .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
        Ok(contact_request
            .properties()
            .contains_key(ENCRYPTED_SHIELDED_ADDRESS))
    }
}

/// A test's own answer to [`IdentityWallet::contact_shielded_address`], for tests of how
/// an address a request carries is read, stored and forgotten. They need a request that
/// carries a known address and not the wallet state the derivation takes it from, which
/// `shielded/contact_address_tests.rs` covers.
#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;

    thread_local! {
        static GIVEN: Cell<Option<[u8; 43]>> = const { Cell::new(None) };
    }

    /// While it lives, every contact request sent from this thread carries the address.
    pub(crate) struct GivenAddress(());

    /// Have every contact request this thread sends carry `address`.
    pub(crate) fn give_every_contact(address: [u8; 43]) -> GivenAddress {
        GIVEN.set(Some(address));
        GivenAddress(())
    }

    impl Drop for GivenAddress {
        fn drop(&mut self) {
            GIVEN.set(None);
        }
    }

    pub(super) fn given() -> Option<[u8; 43]> {
        GIVEN.get()
    }
}
