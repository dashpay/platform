//! Top-up an identity from platform addresses.

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::signer::Signer;
use dpp::prelude::Identifier;

use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

use key_wallet::PlatformP2PKHAddress;

use crate::error::PlatformWalletError;
use crate::PlatformAddressChangeSet;

use super::*;

// ---------------------------------------------------------------------------
// Top-up from platform addresses
// ---------------------------------------------------------------------------

impl IdentityWallet {
    /// Top up an identity by spending platform address balances.
    ///
    /// Uses the `TopUpIdentityFromAddresses` SDK trait. Address nonces are
    /// looked up automatically.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The identity to top up.
    /// * `inputs` - Map of platform addresses to credit amounts to spend.
    /// * `address_signer` - Produces ECDSA signatures for the input
    ///   [`PlatformAddress`]es. Construction is the caller's concern —
    ///   seed-backed, hardware, FFI trampoline, whatever — the wallet
    ///   struct carries no key material itself.
    pub async fn top_up_from_addresses<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        identity_id: &Identifier,
        inputs: BTreeMap<PlatformAddress, Credits>,
        address_signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Credits, PlatformWalletError> {
        let identity = {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            let manager = &info.identity_manager;
            manager
                .identity(identity_id)
                .map(|m| m.identity.clone())
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?
        };

        let (address_infos, new_balance) = identity
            .top_up_from_addresses(&self.sdk, inputs, address_signer, settings)
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "Failed to top up identity from addresses: {}",
                    e
                ))
            })?;

        // Update the identity's balance in the local manager and
        // queue the snapshot so the new balance survives relaunch.
        // See the comment on `top_up` for rationale on driving the
        // persister directly from the call site instead of through
        // a dedicated `ManagedIdentity::set_balance` method.
        {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                crate::error::PlatformWalletError::WalletNotFound(
                    "Wallet info not found in wallet manager".to_string(),
                )
            })?;
            if let Some(managed) = info.identity_manager.managed_identity_mut(identity_id) {
                managed.identity.set_balance(new_balance);
                if let Err(e) = self.persister.store(managed.snapshot_changeset().into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist identity balance update after top_up_from_addresses"
                    );
                }
            }

            // Reconcile the spent platform-address balances from the proof.
            //
            // The SDK trait returns proof-attested `AddressInfos` carrying the
            // new on-chain balance + bumped nonce of every address we just
            // spent from. Write those back into the wallet's platform accounts
            // and persist them — exactly as the sibling `fund_from_asset_lock`
            // path does via `write_address_balances_changeset`.
            //
            // Without this, the local platform-address balances stay frozen at
            // their pre-top-up values: the wallet keeps displaying a stale
            // "Platform Balance", and the next top-up's input selection
            // over-selects the now-drained addresses, so Drive rejects the
            // transition with "Insufficient combined address balances" even
            // though the UI shows ample funds.
            //
            // The spent addresses are already funded, so the `None` key source
            // is safe — `set_address_credit_balance` only consults it on a
            // `0 -> funded` transition (gap-limit maintenance), never on the
            // decrement a spend produces.
            let mut addr_cs = PlatformAddressChangeSet::default();
            for account in info.core_wallet.all_platform_payment_managed_accounts_mut() {
                let account_index = account.account;
                // The spent addresses that belong to THIS account, mapped to
                // their derivation index.
                let mut owned: BTreeMap<PlatformP2PKHAddress, u32> = BTreeMap::new();
                for (addr, _) in address_infos.iter() {
                    let PlatformAddress::P2pkh(hash) = *addr else {
                        continue;
                    };
                    let p2pkh = PlatformP2PKHAddress::new(hash);
                    if let Some(index) =
                        account
                            .addresses
                            .addresses
                            .iter()
                            .find_map(|(&idx, ainfo)| {
                                PlatformP2PKHAddress::from_address(&ainfo.address)
                                    .ok()
                                    .filter(|found| *found == p2pkh)
                                    .map(|_| idx)
                            })
                    {
                        owned.insert(p2pkh, index);
                    }
                }
                if owned.is_empty() {
                    continue;
                }
                // Apply each proof-attested post-spend balance in memory.
                for (addr, maybe_info) in address_infos.iter() {
                    let PlatformAddress::P2pkh(hash) = *addr else {
                        continue;
                    };
                    let p2pkh = PlatformP2PKHAddress::new(hash);
                    if !owned.contains_key(&p2pkh) {
                        continue;
                    }
                    let balance = maybe_info.as_ref().map_or(0, |ai| ai.balance);
                    account.set_address_credit_balance(p2pkh, balance, None);
                }
                addr_cs.addresses.extend(
                    crate::wallet::platform_addresses::build_platform_address_persistence_entries(
                        self.wallet_id,
                        account_index,
                        &owned,
                        address_infos.iter().map(|(a, i)| (a, i.as_ref())),
                    ),
                );
            }
            if !addr_cs.addresses.is_empty() {
                if let Err(e) = self.persister.store(addr_cs.into()) {
                    tracing::error!(
                        identity = %identity_id,
                        error = %e,
                        "Failed to persist platform-address reconciliation after \
                         top_up_from_addresses; in-memory balances are updated but durable \
                         rows stay stale until the next platform-address sync"
                    );
                }
            }
        }

        Ok(new_balance)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dash_sdk::query_types::AddressInfo;
    use dpp::address_funds::PlatformAddress;
    use key_wallet::PlatformP2PKHAddress;

    /// Pins the balance reconciliation that `top_up_from_addresses` MUST
    /// perform on its success path. When a top-up spends platform addresses,
    /// the SDK returns proof-attested `address_infos` carrying the *new*
    /// (decremented) on-chain balance and bumped nonce of every address we
    /// spent from. Those must be written back to the wallet's local
    /// platform-address balances — exactly as the sibling
    /// `fund_from_asset_lock` path does via `write_address_balances_changeset`.
    ///
    /// Production report (2026-06-27): `top_up_from_addresses` discarded the
    /// returned `address_infos`, so the local platform-address balances stayed
    /// frozen at their pre-top-up values. The wallet kept displaying the stale
    /// "Platform Balance", and the next top-up's greedy input selection
    /// over-selected those now-drained addresses, so Drive rejected the
    /// transition with "Insufficient combined address balances: total
    /// available is less than required …" even though the UI showed ample
    /// funds.
    ///
    /// This test exercises the shared reconciliation helper the fix routes the
    /// top-up path through; a spent address whose proof balance is now 5 must
    /// produce a persistence entry of 5 (the on-chain truth), never the stale
    /// pre-spend value.
    #[test]
    fn top_up_records_post_spend_address_balance_not_stale() {
        let wallet_id = [0xCDu8; 32];
        let account_index = 0u32;

        // An owned platform address we spent FROM during the top-up. The
        // wallet locally believed it held a large balance; the proof attests
        // the post-spend balance is now 5 credits, nonce bumped to 4.
        let spent_hash = [0x11u8; 20];
        let spent = PlatformP2PKHAddress::new(spent_hash);
        let spent_addr = PlatformAddress::P2pkh(spent_hash);

        let mut owned: BTreeMap<PlatformP2PKHAddress, u32> = BTreeMap::new();
        owned.insert(spent, 3);

        let post_spend = AddressInfo {
            address: spent_addr,
            nonce: 4,
            balance: 5,
        };
        let address_infos: BTreeMap<PlatformAddress, Option<AddressInfo>> =
            [(spent_addr, Some(post_spend))].into_iter().collect();

        let entries = crate::wallet::platform_addresses::build_platform_address_persistence_entries(
            wallet_id,
            account_index,
            &owned,
            address_infos.iter().map(|(a, i)| (a, i.as_ref())),
        );

        assert_eq!(
            entries.len(),
            1,
            "the spent owned address must get a balance entry"
        );
        let entry = &entries[0];
        assert_eq!(entry.address, spent);
        assert_eq!(
            entry.address_index, 3,
            "must keep the address's real derivation index"
        );
        assert_eq!(
            entry.funds.balance, 5,
            "must record the proof's post-spend balance, not the stale pre-spend value"
        );
        assert_eq!(entry.funds.nonce, 4, "must record the bumped nonce");
    }
}
