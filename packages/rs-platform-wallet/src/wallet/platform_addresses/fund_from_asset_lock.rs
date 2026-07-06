//! Orchestrated platform-address funding from a Core asset lock.
//!
//! Mirrors `IdentityWallet::register_identity_with_funding` from the
//! identity-side flow but credits Platform addresses with the asset
//! lock's value via an `AddressFundingFromAssetLockTransition` instead
//! of creating an identity.
//!
//! ## Pipeline
//!
//! 1. **Pre-flight** — exactly-one-`None`-recipient invariant; each
//!    address must belong to the supplied platform-payment account.
//! 2. **Resolve funding** — delegate to the shared
//!    [`AssetLockManager::resolve_funding_with_is_timeout_fallback`].
//!    `FromWalletBalance` builds an asset-lock tx out of the
//!    `AssetLockAddressTopUp` BIP44 family and waits for IS/CL;
//!    `FromExistingAssetLock` resumes from a tracked outpoint.
//! 3. **Submit** — `addresses.top_up_with_signers(...)` inside the
//!    shared `submit_with_cl_height_retry` wrapper. IS→CL fallback
//!    fires both on Core-side timeout (resolver returns `IsTimeout`)
//!    and on Platform-side IS rejection
//!    (`is_instant_lock_proof_invalid`).
//! 4. **Bookkeeping + cleanup** — write each recipient's new credit
//!    balance into `ManagedPlatformAccount` and emit a
//!    `PlatformAddressChangeSet`; then `consume_asset_lock` the
//!    tracked outpoint so the row is marked `Consumed` (terminal)
//!    and dropped from the in-memory tracked-lock map.

use crate::wallet::asset_lock::orchestration::{
    out_point_from_proof, submit_with_cl_height_retry, AssetLockFunding, FundingResolution,
    ResolvedFunding,
};
use crate::wallet::PlatformAddressWallet;
use crate::{error::is_instant_lock_proof_invalid, PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use dash_sdk::query_types::AddressInfos;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::PlatformP2PKHAddress;
use std::collections::BTreeMap;

impl PlatformAddressWallet {
    /// Fund platform addresses from a Core L1 asset lock, with the
    /// asset-lock proof signed by an external `key_wallet::signer::Signer`.
    ///
    /// This is the orchestrated entry point: it covers the full
    /// build → broadcast → wait-for-IS-or-CL → submit-with-CL-retry →
    /// IS→CL-fallback → consume pipeline. The host never sees the
    /// asset-lock private key — both Core-side derivation (inside the
    /// asset-lock manager) and ST-side signing
    /// (`StateTransition::sign_with_core_signer`) go through
    /// `asset_lock_signer`, which atomically derives + signs +
    /// zeroises inside its trust boundary.
    ///
    /// # Arguments
    ///
    /// * `funding` — How to source the funding asset lock. `FromWalletBalance`
    ///   builds a fresh asset lock from Core UTXOs; `FromExistingAssetLock`
    ///   resumes from a tracked outpoint (after app relaunch or a stuck
    ///   broadcast).
    /// * `platform_account_index` — Platform payment account whose
    ///   addresses receive credits. Used for both the membership
    ///   pre-flight and the post-success balance write.
    /// * `addresses` — Map from recipient `PlatformAddress` to optional
    ///   amount in credits. Exactly one entry must be `None` — the
    ///   remainder-after-fees-and-explicit-outputs recipient (the lock
    ///   is consumed in full, so a remainder bucket is mandatory).
    /// * `fee_strategy` — Per-step fee-deduction strategy applied to
    ///   the address-funding transition.
    /// * `address_signer` — Signs per-input `AddressWitness` for any
    ///   additional inputs from existing platform addresses (today
    ///   none — combining external inputs with an asset-lock proof is
    ///   not exercised here, but `AddressFundingFromAssetLockTransitionV0`
    ///   does allow it).
    /// * `asset_lock_signer` — Signs the outer state-transition ECDSA
    ///   signature against the asset lock's credit-output key. The
    ///   wallet struct itself carries no key material; signing is
    ///   atomic + zeroising inside this signer.
    /// * `settings` — `PutSettings::user_fee_increase` is threaded
    ///   through to the ST builder. The CL-height retry wrapper bumps
    ///   this value on consensus-10506 to bypass Tenderdash's
    ///   invalid-tx hash cache; the caller's initial value is the
    ///   starting point.
    ///
    /// # Latency budget
    ///
    /// There is intentionally **no ceiling** on this call: a broadcast
    /// asset lock is committed on-chain, and its ChainLock is
    /// deterministic finality that will eventually arrive, so the flow
    /// waits for finality however long it takes rather than reporting a
    /// spurious failure. The components are:
    /// - 300s IS-wait inside the resolver's
    ///   `create_funded_asset_lock_proof` (`AssetLockManager`'s
    ///   InstantSend-preference window before falling back to ChainLock).
    /// - **Unbounded** ChainLock fallback
    ///   (`upgrade_to_chain_lock_proof(None)`) — waits for the ChainLock
    ///   indefinitely (testnet ChainLocks can take ~15min).
    /// - 210s CL-height retry budget (`CL_HEIGHT_RETRY_BUDGET`) per
    ///   `submit_with_cl_height_retry` wrapper.
    /// - Up to two passes through the submit wrapper on the
    ///   IS-rejection path: one for the IS proof, one for the
    ///   upgraded CL proof.
    ///
    /// Happy-path wall time on a healthy testnet is single-digit
    /// seconds (IS-lock typically arrives within 3s of broadcast,
    /// CL-height retry never fires). The unbounded wait only bites when
    /// InstantSend never propagates and the caller must wait out the
    /// slower ChainLock. Callers run this off the main thread and never
    /// cancel it (see the Cancellation note below).
    ///
    /// # Cancellation
    ///
    /// This function is NOT cancellation-safe. The two underlying
    /// retry loops (`submit_with_cl_height_retry` and the
    /// resolver's internal `wait_for_proof`) use
    /// `tokio::time::sleep` / `tokio::sync::Notify` without
    /// structured cancellation hooks. If the caller drops the
    /// returned future:
    /// - Any bumped `user_fee_increase` is lost; the next attempt
    ///   starts from the caller-supplied value, which may hit
    ///   Tenderdash's invalid-tx cache for the bumped variants.
    /// - In-flight submitted state transitions remain in
    ///   Tenderdash's mempool until they commit or expire.
    /// - The tracked asset lock stays at its last-observed status
    ///   (`Broadcast` / `InstantSendLocked` / `ChainLocked`) until
    ///   either `consume_asset_lock` completes or the next resume
    ///   advances it.
    ///
    /// The Swift `AddressFundFromAssetLockController.task` field deliberately
    /// does not call `.cancel()` to avoid these partial-state
    /// outcomes — the FFI call always runs to completion. UI
    /// dismissal hides the progress view without aborting the
    /// work; resume picks the lock back up via
    /// `FromExistingAssetLock`.
    #[allow(clippy::too_many_arguments)]
    pub async fn fund_from_asset_lock<S, AS>(
        &self,
        funding: AssetLockFunding,
        platform_account_index: u32,
        addresses: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        address_signer: &S,
        asset_lock_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError>
    where
        S: Signer<PlatformAddress> + Send + Sync,
        AS: ::key_wallet::signer::Signer + Send + Sync,
    {
        // Step 1: pre-flight. Failing fast here avoids broadcasting
        // an unfundable asset-lock tx.
        validate_recipient_addresses(self, platform_account_index, &addresses).await?;

        // Step 2: resolve funding. `AssetLockAddressTopUp` selects the
        // BIP44 funding family for the Core asset-lock tx. The
        // `destination_index = 0` argument is unused by this funding
        // type (the resolver only consults it for `IdentityTopUp`),
        // so any value is fine.
        let ResolvedFunding {
            proof,
            path,
            tracked_out_point,
        } = match self
            .asset_locks
            .resolve_funding_with_is_timeout_fallback(
                funding,
                AssetLockFundingType::AssetLockAddressTopUp,
                /* destination_index */ 0,
                asset_lock_signer,
            )
            .await?
        {
            FundingResolution::Resolved(rf) => rf,
            FundingResolution::IsTimeout { out_point } => {
                tracing::warn!(
                    "IS-lock did not propagate within 300s for funded platform-address top-up \
                     (tx {}), falling back to ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                // Re-derive the credit-output path. The lock is now
                // CL-attached; `resume_asset_lock` short-circuits to
                // the existing-proof branch and just hands the path
                // back.
                let (_, path) = self.asset_locks.resume_asset_lock(&out_point, None).await?;
                ResolvedFunding {
                    proof: chain_proof,
                    path,
                    tracked_out_point: Some(out_point),
                }
            }
        };

        // Step 3: submit. Two Platform-side fallback layers (matches
        // `register_identity_with_funding`): CL-height-too-low retries
        // bump `user_fee_increase` to bypass Tenderdash's invalid-tx
        // cache, and IS-lock rejection triggers an IS→CL upgrade on
        // the same outpoint.
        let proof_out_point = out_point_from_proof(&proof);
        // `proof_height` is the broadcast proof's committed block — the
        // height pin for the reconciled absolutes below.
        let (address_infos, proof_height) = match submit_with_cl_height_retry(settings, |s| {
            addresses.top_up_with_signers(
                &self.sdk,
                proof.clone(),
                &path,
                fee_strategy.clone(),
                address_signer,
                asset_lock_signer,
                s,
            )
        })
        .await
        {
            Ok(infos) => infos,
            Err(e) if is_instant_lock_proof_invalid(&e) => {
                let out_point = proof_out_point;
                tracing::warn!(
                    "IS-lock proof rejected by Platform for platform-address top-up (tx {}), \
                     retrying with ChainLock proof",
                    out_point.txid
                );
                let chain_proof = self
                    .asset_locks
                    .upgrade_to_chain_lock_proof(&out_point, None)
                    .await?;
                // Advance the tracked status from `InstantSendLocked`
                // to `ChainLocked` with the upgraded proof attached
                // BEFORE the second submit. If the next call fails
                // (transport blip, fresh CL-height race that
                // exhausts the retry budget), the row accurately
                // reflects the lock's CL-attached state instead of
                // the stale IS proof Platform just rejected. The
                // catch-up scanner / Resume path then has a
                // truthful status to work from.
                let cs = self
                    .asset_locks
                    .advance_asset_lock_status(
                        &out_point,
                        crate::wallet::asset_lock::tracked::AssetLockStatus::ChainLocked,
                        Some(chain_proof.clone()),
                    )
                    .await?;
                self.asset_locks.queue_asset_lock_changeset(cs);
                submit_with_cl_height_retry(settings, |s| {
                    addresses.top_up_with_signers(
                        &self.sdk,
                        chain_proof.clone(),
                        &path,
                        fee_strategy.clone(),
                        address_signer,
                        asset_lock_signer,
                        s,
                    )
                })
                .await
                .map_err(PlatformWalletError::Sdk)?
            }
            Err(e) => return Err(PlatformWalletError::Sdk(e)),
        };

        // Step 4: bookkeeping + cleanup. Write the proof-attested
        // balances back into ManagedPlatformAccount, then consume the
        // tracked asset lock (terminal — marks the row `Consumed` and
        // drops it from the in-memory map).

        // Post-condition: every requested recipient must appear in
        // the proof-attested `address_infos` with a `Some(_)` info.
        // A wholly-empty map, an absent recipient, or a
        // `Some(addr) -> None` entry would each be a DAPI /
        // proof-verifier contract violation, NOT a successful
        // zero-credit funding. We fail loud rather than silently
        // consume the asset lock with no recorded credits for some
        // or all recipients.
        validate_address_infos_complete(&addresses, &address_infos)?;

        // The shared seam applies the proof-attested balances to the
        // managed accounts, updates the provider's sync seed, and
        // persists — without the persist, rows for these recipients
        // stay frozen at pre-top-up values until the next BLAST sync;
        // on a process restart before that sync,
        // `initialize_from_persisted` would seed
        // `account.address_credit_balance` from the stale rows while
        // the asset-lock record is already `Consumed`, leaving
        // `auto_select_inputs` to under-budget and produce
        // protocol-level rejections until a sync repairs them.
        //
        // The seam persists before returning, and the persist MUST
        // happen before `consume_asset_lock` so we never have a
        // Consumed lock paired with a stale balance row on disk.
        // Persistence errors are logged inside the seam rather than
        // propagated: Platform already accepted the transition, and a
        // persistence hiccup shouldn't mask that.
        //
        // ADDR-09: every recipient of an asset-lock top-up is credited via
        // an on-chain `AddBalanceToAddress` DELTA at exactly this proof's
        // block height. The committed absolutes carry `proof_height` as
        // their height pin (`AddressFunds::as_of_height`), so the sync's
        // apply loops drop that delta (and any older one) instead of
        // re-applying it on top → no `X + X = 2X` double-count, on
        // incremental AND full-scan passes alike.
        //
        // Use the persistence-reporting variant: marking the lock
        // `Consumed` below is irreversible, so it MUST be gated on the
        // reconciled balances actually reaching disk. `persisted` is
        // false ONLY when the in-memory balances were updated but the
        // durable write failed — exactly the case where a Consumed lock
        // would pair with stale rows and under-budget the next spend
        // after a restart.
        let (cs, persisted) = self
            .reconcile_address_infos_with_persistence(
                &address_infos,
                proof_height,
                "fund from asset lock",
            )
            .await;

        if let Some(out_point) = tracked_out_point {
            if !persisted {
                // The proof-attested balances were applied in memory but
                // did not reach disk. Leave the lock non-Consumed: it
                // stays in the Resumable Funding list, and a user Resume
                // gets Platform's deterministic "lock already consumed"
                // rejection — the same benign recovery path as a failed
                // consume below — while the next platform-address sync
                // repairs the stale rows. Consuming here would strand the
                // lock as Consumed over durable balances that under-report
                // the credit.
                tracing::error!(
                    outpoint = %out_point,
                    "skipping consume_asset_lock: the reconciled balance changeset \
                     was not durably stored; the lock stays non-Consumed (Resumable) \
                     rather than pairing a Consumed lock with stale balance rows on disk"
                );
                return Ok(cs);
            }
            // Platform DID accept the top-up — propagating an Err
            // here would misreport the protocol outcome, since the
            // caller's recipient(s) already have credits attested
            // by the proof we just decoded. But: the lock row stays
            // in non-Consumed status, which means it will surface
            // in the Resumable Funding list and the user could try
            // to fund it again — Platform would deterministically
            // reject the duplicate ST with "lock already consumed".
            //
            // The expected failure mode is `WalletNotFound` (the
            // wallet handle vanished between submit-success and
            // this cleanup). Log that as a warn — the user-visible
            // recovery path (Resume + Platform's deterministic
            // rejection) is benign. Anything else is an unexpected
            // invariant violation — log as `error` so it shows up
            // in operational dashboards.
            if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                match &e {
                    PlatformWalletError::WalletNotFound(_) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock: wallet handle vanished after successful Platform submit"
                        );
                    }
                    _ => {
                        tracing::error!(
                            outpoint = %out_point,
                            error = %e,
                            "consume_asset_lock failed unexpectedly after successful Platform submit; \
                             the lock row stays non-Consumed and will surface as Resumable. \
                             A user Resume on it will be rejected by Platform with 'lock already consumed'."
                        );
                    }
                }
            }
        }

        Ok(cs)
    }
}

/// Pre-flight check for the recipient address map:
/// - Non-empty
/// - Exactly one `None`-amount entry (the remainder recipient)
/// - All addresses are P2PKH and belong to the specified platform-payment account
async fn validate_recipient_addresses(
    wallet: &PlatformAddressWallet,
    platform_account_index: u32,
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
) -> Result<(), PlatformWalletError> {
    if addresses.is_empty() {
        return Err(PlatformWalletError::AddressOperation(
            "fund_from_asset_lock requires at least one recipient address".to_string(),
        ));
    }

    let none_count = addresses.values().filter(|v| v.is_none()).count();
    if none_count != 1 {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Exactly one address must have None balance (the funding recipient), found {}",
            none_count
        )));
    }

    let wm = wallet.wallet_manager.read().await;
    let info = wm.get_wallet_info(&wallet.wallet_id).ok_or_else(|| {
        PlatformWalletError::WalletNotFound(format!(
            "Wallet {:?} not found in wallet manager",
            hex::encode(wallet.wallet_id)
        ))
    })?;
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index(platform_account_index)
        .ok_or_else(|| {
            PlatformWalletError::AddressSync(format!(
                "No platform payment account at index {}",
                platform_account_index
            ))
        })?;
    for addr in addresses.keys() {
        let PlatformAddress::P2pkh(hash) = addr else {
            return Err(PlatformWalletError::AddressOperation(
                "Only P2PKH addresses are supported".to_string(),
            ));
        };
        let p2pkh = PlatformP2PKHAddress::new(*hash);
        if !account.contains_platform_address(&p2pkh) {
            return Err(PlatformWalletError::AddressNotFound(format!(
                "Address {} does not belong to platform account index {}",
                p2pkh, platform_account_index
            )));
        }
    }
    Ok(())
}

/// Post-submit guard: confirm the proof-attested `address_infos` carry a
/// usable `AddressInfo` for every requested recipient.
///
/// A wholly-empty map, an entry whose value is `None`, or a recipient
/// not present in the map at all are each a DAPI / proof-verifier
/// contract violation — Platform accepted the transition, but the
/// proof omits credits for the caller's recipients. Returning `Ok`
/// here would let `consume_asset_lock` terminally destroy the only
/// resumable record for the L1 funding outpoint while one or more
/// recipients silently lose credits, which (since asset locks are
/// non-refundable) is permanent value loss.
fn validate_address_infos_complete(
    addresses: &BTreeMap<PlatformAddress, Option<Credits>>,
    address_infos: &AddressInfos,
) -> Result<(), PlatformWalletError> {
    let expected_recipient_count = addresses.len();
    if address_infos.is_empty() {
        return Err(PlatformWalletError::AddressSync(format!(
            "Address-funding ST succeeded but the proof returned no address infos \
             (expected {} recipient(s)); refusing to consume the asset lock with \
             no recorded credits",
            expected_recipient_count
        )));
    }

    let missing_recipients: Vec<String> = addresses
        .keys()
        .filter_map(|address| match address_infos.get(address) {
            Some(Some(_)) => None,
            _ => Some(format!("{address:?}")),
        })
        .collect();

    if !missing_recipients.is_empty() {
        return Err(PlatformWalletError::AddressSync(format!(
            "Address-funding ST succeeded but the proof omitted usable AddressInfo \
             for recipient(s): {}",
            missing_recipients.join(", ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::query_types::AddressInfo;

    fn p2pkh(b: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([b; 20])
    }

    fn info(addr: PlatformAddress) -> AddressInfo {
        AddressInfo {
            address: addr,
            balance: 1_000,
            nonce: 0,
        }
    }

    #[test]
    fn rejects_empty_address_infos_for_non_empty_recipients() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), None);
        let address_infos: AddressInfos = AddressInfos::new();
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("empty address_infos must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("returned no address infos"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn rejects_recipient_with_none_address_info() {
        // The proof contract violation we actually hit in practice:
        // proof carries the recipient key but with a `None` value.
        // Pre-fix, this slipped past the inline `is_empty()` guard
        // and the asset lock got consumed regardless.
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), None);
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("None info must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("omitted usable AddressInfo"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn rejects_recipient_absent_from_address_infos() {
        // Multi-recipient case: proof present for one address but
        // missing the other entirely. Without the per-recipient
        // check, this would also silently consume the lock with
        // partial crediting.
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), Some(500));
        addresses.insert(p2pkh(2), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), Some(info(p2pkh(1))));
        let err = validate_address_infos_complete(&addresses, &address_infos)
            .expect_err("missing recipient must be rejected");
        let msg = format!("{}", err);
        assert!(
            msg.contains("omitted usable AddressInfo"),
            "unexpected: {msg}"
        );
    }

    #[test]
    fn accepts_every_recipient_with_some_address_info() {
        let mut addresses = BTreeMap::new();
        addresses.insert(p2pkh(1), Some(500));
        addresses.insert(p2pkh(2), None);
        let mut address_infos: AddressInfos = AddressInfos::new();
        address_infos.insert(p2pkh(1), Some(info(p2pkh(1))));
        address_infos.insert(p2pkh(2), Some(info(p2pkh(2))));
        validate_address_infos_complete(&addresses, &address_infos)
            .expect("complete proof must pass");
    }
}
