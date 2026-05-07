//! Test-wallet factory plus the [`SetupGuard`] returned by
//! [`super::setup`]. Every wallet is registered in the persistent
//! registry BEFORE returning to the test body, so a panic between
//! `setup` and `teardown` leaves a recoverable trail for the next
//! startup's sweep.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use key_wallet::wallet::initialization::{
    PlatformPaymentAccountSpec, WalletAccountCreationOptions,
};
use key_wallet::Network;
use platform_wallet::wallet::persister::NoPlatformPersistence;
use platform_wallet::wallet::platform_addresses::InputSelection;
use platform_wallet::{
    PlatformAddressChangeSet, PlatformWallet, PlatformWalletError, PlatformWalletManager,
};
use rand::rngs::OsRng;
use rand::RngCore;

use simple_signer::signer::SimpleSigner;

use super::harness::E2eContext;
use super::registry::{EntryStatus, PersistentTestWalletRegistry, RegistryEntry, WalletSeedHash};
use super::signer::{derive_identity_key, SeedBackedIdentitySigner};
use super::wait::wait_for_identity_balance;
use super::wait_hub::WaitEventHub;
use super::{make_platform_signer, FrameworkError, FrameworkResult};

/// DIP-17 default PlatformPayment account spec — pinned to
/// `PlatformPaymentAccountSpec` field defaults so a struct-shape change
/// upstream fails to compile here.
const DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC: PlatformPaymentAccountSpec =
    PlatformPaymentAccountSpec {
        account: 0,
        key_class: 0,
    };

pub(super) const DEFAULT_ACCOUNT_INDEX_PUB: u32 = DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC.account;
pub(super) const DEFAULT_KEY_CLASS_PUB: u32 = DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC.key_class;

/// `PlatformPaymentAccountKey` for the default DIP-17 account, derived
/// from the canonical [`PlatformPaymentAccountSpec`] in `key_wallet`.
fn default_platform_payment_account_key() -> PlatformPaymentAccountKey {
    let PlatformPaymentAccountSpec { account, key_class } = PlatformPaymentAccountSpec::default();
    PlatformPaymentAccountKey { account, key_class }
}

/// Per-test wallet handle. Exposes the high-level operations test
/// cases reach for (`next_unused_address`, `transfer`, `balances`,
/// `sync_balances`) without leaking the underlying `PlatformWallet`
/// surface.
pub struct TestWallet {
    seed_bytes: [u8; 64],
    pub(crate) wallet: Arc<PlatformWallet>,
    signer: SimpleSigner,
    /// Cloned from the [`E2eContext`]; backs
    /// [`super::wait::wait_for_balance`].
    wait_hub: Arc<WaitEventHub>,
}

impl std::fmt::Debug for TestWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestWallet")
            .field("wallet_id", &hex::encode(self.wallet.wallet_id()))
            .finish_non_exhaustive()
    }
}

impl TestWallet {
    /// Create a fresh-seeded test wallet, register with the
    /// manager, and eagerly initialise its platform-address
    /// provider so `next_unused_address` / `transfer` work
    /// immediately on return.
    ///
    /// The caller passes `seed_bytes` (typically via `OsRng`) so the
    /// registry can persist them BEFORE the wallet is returned —
    /// a crashed test still has a recoverable record.
    pub async fn create(
        manager: &Arc<PlatformWalletManager<NoPlatformPersistence>>,
        seed_bytes: [u8; 64],
        network: Network,
        wait_hub: Arc<WaitEventHub>,
    ) -> FrameworkResult<Self> {
        let wallet = manager
            .create_wallet_from_seed_bytes(
                network,
                seed_bytes,
                WalletAccountCreationOptions::Default,
                None,
            )
            .await
            .map_err(wallet_err)?;
        // Force the lazy platform-address init now so test code
        // doesn't see a surprise first-use latency hit.
        wallet.platform().initialize().await;
        // QA-002: pre-consume the slot-0 receive address on the
        // default DIP-17 (account=0, key_class=0) account so the test
        // wallet's first `next_unused_address()` returns index 1
        // instead of index 0. The bank pins
        // `BankWallet::primary_receive_address` to slot-0 of the same
        // account/key_class via `derive_platform_address_at_index`,
        // and (when the test seed shares derivation parameters with
        // the bank seed) both wallets resolve that slot to the same
        // P2PKH. When a prior-test cleanup sweep + the current test's
        // `bank.fund_address` both target that hash in the same
        // block, drive-abci's recent-zone feed correctly merges them
        // via `AddToCredits + AddToCredits = saturating_add` —
        // inflating the BLAST sync surface relative to the value the
        // registration call attributes to its own input. See
        // `/tmp/qa002-confirmed.md`.
        consume_platform_address_index_zero(&wallet).await?;
        let signer = make_platform_signer(&seed_bytes, network)?;
        Ok(Self {
            seed_bytes,
            wallet,
            signer,
            wait_hub,
        })
    }

    /// Stable wallet id used as the registry key.
    pub fn id(&self) -> WalletSeedHash {
        self.wallet.wallet_id()
    }

    /// 64-byte seed used to derive this wallet (persisted in the
    /// registry so a sweep can reconstruct the wallet).
    pub fn seed_bytes(&self) -> [u8; 64] {
        self.seed_bytes
    }

    /// Underlying `PlatformWallet` — for tests that reach into
    /// identity / token / core APIs.
    pub fn platform_wallet(&self) -> &Arc<PlatformWallet> {
        &self.wallet
    }

    /// Seed-backed address signer used by `transfer`; tests that
    /// broadcast transitions via the SDK directly can pass it in.
    /// Implements `Signer<PlatformAddress>` directly.
    pub fn address_signer(&self) -> &SimpleSigner {
        &self.signer
    }

    /// Process-shared event hub — backs
    /// [`super::wait::wait_for_balance`].
    pub fn wait_hub(&self) -> &Arc<WaitEventHub> {
        &self.wait_hub
    }

    /// Next unused receive address on the wallet's default
    /// platform-payment account. Pool advances only after a sync
    /// observes an inbound credit on the prior address; a freshly
    /// returned address has balance `0` until the next sync sees it
    /// funded. Returns a new address if the gap window is exhausted.
    pub async fn next_unused_address(&self) -> FrameworkResult<PlatformAddress> {
        self.wallet
            .platform()
            .next_unused_receive_address(default_platform_payment_account_key())
            .await
            .map_err(wallet_err)
    }

    /// Run a BLAST sync pass and refresh balances for every
    /// tracked address.
    pub async fn sync_balances(&self) -> FrameworkResult<()> {
        self.wallet
            .platform()
            .sync_balances(None)
            .await
            .map(|_| ())
            .map_err(wallet_err)
    }

    /// Snapshot of cached balances per tracked address. Reflects
    /// the last `sync_balances` — call it first if you need a fresh
    /// view.
    pub async fn balances(&self) -> BTreeMap<PlatformAddress, Credits> {
        self.wallet
            .platform()
            .addresses_with_balances()
            .await
            .into_iter()
            .collect()
    }

    /// Total credits across every tracked address.
    pub async fn total_credits(&self) -> Credits {
        self.wallet.platform().total_credits().await
    }

    /// Lock-free Core (Layer-1) confirmed balance in duffs, sourced
    /// from the atomic updated by SPV. Test helper — not
    /// transactionally consistent with the wallet's UTXO set.
    pub fn core_balance_confirmed(&self) -> u64 {
        self.wallet.balance().confirmed()
    }

    /// Sweep `amount` Core duffs from this wallet's BIP-44 account 0
    /// to `target`. Thin wrapper over [`super::bank::core_send`] —
    /// builds, signs, and broadcasts via [`SpvBroadcaster`]. Returns
    /// the broadcast `Txid`.
    ///
    /// Test-only helper: the framework's
    /// [`super::cleanup::teardown_one`] / `sweep_orphans` paths
    /// already drain Core funds through the same `core_send` free
    /// function, so individual tests rarely need this directly. Add
    /// it for cases that explicitly want to broadcast a Core send
    /// from a test wallet without going through teardown.
    pub async fn sweep_core_to(
        &self,
        target: &dashcore::Address,
        amount: u64,
    ) -> FrameworkResult<dashcore::Txid> {
        super::bank::core_send(&self.wallet, target, amount).await
    }

    /// Transfer credits to one or more outputs. Auto-selects inputs
    /// from the default account and uses [`default_fee_strategy`]
    /// (reduce output #0). `outputs` maps each recipient address
    /// to its credit amount.
    pub async fn transfer(
        &self,
        outputs: BTreeMap<PlatformAddress, Credits>,
    ) -> FrameworkResult<PlatformAddressChangeSet> {
        self.wallet
            .platform()
            .transfer(
                DEFAULT_ACCOUNT_INDEX_PUB,
                InputSelection::Auto,
                outputs,
                default_fee_strategy(),
                Some(PlatformVersion::latest()),
                &self.signer,
            )
            .await
            .map_err(wallet_err)
    }

    /// Like [`Self::transfer`] but with an explicit input list
    /// (`InputSelection::Explicit`). Used by tests that need to
    /// drive the SDK's address-funds path without the wallet's
    /// `auto_select_inputs` step — typically the negative variants
    /// of PA-002 that probe insufficient-funds behaviour on a
    /// caller-chosen input set.
    pub async fn transfer_with_inputs(
        &self,
        outputs: BTreeMap<PlatformAddress, Credits>,
        inputs: BTreeMap<PlatformAddress, Credits>,
    ) -> FrameworkResult<PlatformAddressChangeSet> {
        self.wallet
            .platform()
            .transfer(
                DEFAULT_ACCOUNT_INDEX_PUB,
                InputSelection::Explicit(inputs),
                outputs,
                default_fee_strategy(),
                Some(PlatformVersion::latest()),
                &self.signer,
            )
            .await
            .map_err(wallet_err)
    }

    /// Like [`Self::transfer_with_inputs`] but additionally returns
    /// the canonical bytes of an `AddressFundsTransferTransition`
    /// built with the same inputs / outputs / fee strategy.
    ///
    /// Used by replay-safety tests (PA-006): re-submit the captured
    /// bytes via `sdk.broadcast_state_transition` and assert the
    /// platform rejects the duplicate. The captured bytes are taken
    /// from a sibling build (separate nonce fetch, separate signing
    /// pass) — they are NOT byte-equal to the broadcast transition
    /// because ECDSA signing is non-deterministic (no RFC 6979 enforced
    /// here). Both transitions share identical address nonces: the
    /// sibling capture never broadcasts, so on-chain state between the
    /// two builds is unchanged. For PA-006 this means re-broadcast is
    /// rejected on nonce-duplicate detection (not content-hash duplicate
    /// detection); assertions should target the nonce-duplicate
    /// rejection reason, or capture bytes from the production submission
    /// so the replayed transition shares both nonce and signature.
    ///
    /// The caller's `inputs` map supplies the **set of input addresses**;
    /// per-address amounts are recomputed by [`balance_explicit_inputs`]
    /// so that `Σ inputs == Σ outputs` (the protocol's strict balance
    /// check on `AddressFundsTransferTransition`). With
    /// `[ReduceOutput(0)]`, the chain-time fee is taken from output 0
    /// at execution; the encoded transition itself must still balance
    /// pre-fee. Callers may pass `address.balance` as a placeholder —
    /// it is only used as a relative weight when distributing across
    /// multiple input addresses.
    pub async fn transfer_capturing_st_bytes(
        &self,
        outputs: BTreeMap<PlatformAddress, Credits>,
        inputs: BTreeMap<PlatformAddress, Credits>,
    ) -> FrameworkResult<(PlatformAddressChangeSet, Vec<u8>)> {
        use dash_sdk::platform::transition::address_inputs::{fetch_inputs_with_nonce, nonce_inc};
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
        use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;

        let platform_version = PlatformVersion::latest();
        let balanced_inputs = balance_explicit_inputs(&inputs, &outputs, platform_version)?;

        // Sibling build for byte capture. Fetches on-chain nonces and
        // bumps them via the public SDK helpers, then signs + serializes.
        // The transition is NEVER broadcast — `transfer_with_inputs`
        // below does its own nonce fetch + sign + broadcast.
        let inputs_with_nonce = fetch_inputs_with_nonce(self.wallet.sdk(), &balanced_inputs)
            .await
            .map_err(|err| FrameworkError::Wallet(format!("nonce fetch: {err}")))?;
        let inputs_with_nonce = nonce_inc(inputs_with_nonce);

        let st = AddressFundsTransferTransition::try_from_inputs_with_signer(
            inputs_with_nonce,
            outputs.clone(),
            default_fee_strategy(),
            &self.signer,
            Default::default(),
            platform_version,
        )
        .await
        .map_err(|err| FrameworkError::Wallet(format!("st build: {err}")))?;
        let bytes = PlatformSerializable::serialize_to_bytes(&st)
            .map_err(|err| FrameworkError::Wallet(format!("st serialize: {err}")))?;

        // Production transfer with the same explicit inputs. Wallet
        // caches + chain state advance per the canonical path.
        let cs = self.transfer_with_inputs(outputs, balanced_inputs).await?;
        Ok((cs, bytes))
    }

    /// Like [`Self::transfer_capturing_st_bytes`] but does NOT
    /// broadcast a parallel production transition. Returns just the
    /// canonical signed bytes of an `AddressFundsTransferTransition`
    /// built against the supplied inputs / outputs.
    ///
    /// Used by PA-006b (concurrent identical broadcasts): the
    /// captured bytes carry a fresh on-chain nonce (no prior
    /// production build has consumed it), so two `tokio::spawn`
    /// tasks each calling `state_transition.broadcast(sdk, None)`
    /// race for one slot.
    pub async fn build_transfer_st_bytes(
        &self,
        outputs: BTreeMap<PlatformAddress, Credits>,
        inputs: BTreeMap<PlatformAddress, Credits>,
    ) -> FrameworkResult<Vec<u8>> {
        use dash_sdk::platform::transition::address_inputs::{fetch_inputs_with_nonce, nonce_inc};
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::address_funds_transfer_transition::methods::AddressFundsTransferTransitionMethodsV0;
        use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;

        let platform_version = PlatformVersion::latest();
        let balanced_inputs = balance_explicit_inputs(&inputs, &outputs, platform_version)?;

        let inputs_with_nonce = fetch_inputs_with_nonce(self.wallet.sdk(), &balanced_inputs)
            .await
            .map_err(|err| FrameworkError::Wallet(format!("nonce fetch: {err}")))?;
        let inputs_with_nonce = nonce_inc(inputs_with_nonce);

        let st = AddressFundsTransferTransition::try_from_inputs_with_signer(
            inputs_with_nonce,
            outputs,
            default_fee_strategy(),
            &self.signer,
            Default::default(),
            PlatformVersion::latest(),
        )
        .await
        .map_err(|err| FrameworkError::Wallet(format!("st build: {err}")))?;
        PlatformSerializable::serialize_to_bytes(&st)
            .map_err(|err| FrameworkError::Wallet(format!("st serialize: {err}")))
    }

    /// Network the wallet operates against. Mirrors `wallet.sdk().network`.
    fn network(&self) -> Network {
        self.wallet.sdk().network
    }

    /// Register a new identity, funded entirely from this wallet's
    /// platform-address balances.
    ///
    /// The helper:
    /// 1. Accepts a caller-provided `funding_address` (the caller is
    ///    responsible for funding it — typically via
    ///    `bank.fund_address` + [`super::wait::wait_for_balance`]
    ///    before this call). No pre-check is performed; passing an
    ///    under-funded address surfaces as a registration failure
    ///    downstream rather than a clear error here.
    /// 2. Derives MASTER + HIGH ECDSA auth keys at DIP-9 slot
    ///    `(identity_index, 0)` and `(identity_index, 1)`, a
    ///    TRANSFER + CRITICAL ECDSA key at slot `(identity_index, 2)`,
    ///    and an AUTHENTICATION + CRITICAL ECDSA key at slot
    ///    `(identity_index, 3)`. The TRANSFER key is required by DPP
    ///    (`identity_credit_transfer_transition` v0_methods.rs:63-83)
    ///    for credit-transfer transitions; without it id_003 / id_005
    ///    / id-sweep all fail with "no transfer public key". The
    ///    CRITICAL auth key is required for token-batch state
    ///    transitions (mint, burn, transfer, freeze, unfreeze,
    ///    destroy_frozen, pause/resume, set_price, purchase,
    ///    update_config) — DPP's `TokenBaseTransition` accepts ONLY
    ///    `SecurityLevel::CRITICAL` and rejects HIGH with
    ///    `InvalidSignaturePublicKeySecurityLevelError`.
    /// 3. Builds a placeholder [`Identity`] populated with those
    ///    four keys.
    /// 4. Calls
    ///    [`IdentityWallet::register_from_addresses`](platform_wallet::wallet::identity::IdentityWallet::register_from_addresses)
    ///    with the funding map `{addr_1 → funding}`.
    /// 5. Waits up to [`DEFAULT_IDENTITY_VISIBILITY_TIMEOUT`] for
    ///    the on-chain balance to reach the post-registration
    ///    threshold.
    pub async fn register_identity_from_addresses(
        &self,
        funding_address: PlatformAddress,
        funding: Credits,
        identity_index: u32,
    ) -> FrameworkResult<RegisteredIdentity> {
        let network = self.network();
        let identity_signer = Arc::new(SeedBackedIdentitySigner::new(
            &self.seed_bytes,
            network,
            identity_index,
        )?);

        // Slot 0 → MASTER, slot 1 → HIGH, slot 2 → TRANSFER, slot 3 →
        // CRITICAL auth. MASTER is required for identity mutation,
        // HIGH covers `DataContractCreate` (which accepts HIGH or
        // CRITICAL) and most credit-balance state transitions,
        // TRANSFER is enforced by DPP for credit transfers (rs-dpp
        // `identity_credit_transfer_transition/v0/v0_methods.rs:63-83`
        // calls `identity.get_first_public_key_matching(Purpose::TRANSFER, ...)`
        // and rejects if absent), and CRITICAL is required for every
        // token-batch transition (`TokenBaseTransition`'s
        // `IdentitySignedV0::security_level_requirement` returns only
        // `SecurityLevel::CRITICAL` — see rs-dpp
        // `state_transition/batch_transition/batched_transition/token_base_transition/identity_signed/v0/`).
        let master_key = derive_identity_key(
            &self.seed_bytes,
            network,
            identity_index,
            0,
            Purpose::AUTHENTICATION,
            SecurityLevel::MASTER,
        )?;
        let high_key = derive_identity_key(
            &self.seed_bytes,
            network,
            identity_index,
            1,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
        )?;
        let transfer_key = derive_identity_key(
            &self.seed_bytes,
            network,
            identity_index,
            2,
            Purpose::TRANSFER,
            SecurityLevel::CRITICAL,
        )?;
        let critical_key = derive_identity_key(
            &self.seed_bytes,
            network,
            identity_index,
            3,
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
        )?;

        // Build the placeholder identity. `id` is recomputed from
        // the input-address map by the SDK at submit time; we set
        // it to `Identifier::default()` per the wallet API contract.
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        let mut public_keys: BTreeMap<KeyID, IdentityPublicKey> = BTreeMap::new();
        public_keys.insert(master_key.id(), master_key.clone());
        public_keys.insert(high_key.id(), high_key.clone());
        public_keys.insert(transfer_key.id(), transfer_key.clone());
        public_keys.insert(critical_key.id(), critical_key.clone());
        let placeholder = Identity::V0(IdentityV0 {
            id: Identifier::default(),
            public_keys,
            balance: 0,
            revision: 0,
        });

        let inputs: BTreeMap<PlatformAddress, Credits> =
            std::iter::once((funding_address, funding)).collect();

        let registered = self
            .wallet
            .identity()
            .register_from_addresses(
                &placeholder,
                inputs,
                None,
                identity_index,
                identity_signer.as_ref(),
                &self.signer,
                None,
            )
            .await
            .map_err(wallet_err)?;

        // The balance check uses a post-fee threshold of `funding /
        // 2` — registration fees on testnet are well below half the
        // funding amount, so this gives us a deterministic "the
        // identity exists and has been credited" assertion without
        // hard-coding a specific fee number that a protocol bump
        // could invalidate.
        wait_for_identity_balance(
            self.wallet.sdk(),
            registered.id(),
            funding / 2,
            DEFAULT_IDENTITY_VISIBILITY_TIMEOUT,
        )
        .await?;

        Ok(RegisteredIdentity {
            id: registered.id(),
            master_key,
            high_key,
            transfer_key,
            critical_key,
            signer: identity_signer,
            identity_index,
            funding,
        })
    }
}

/// Default fee strategy: reduce output #0 by the fee amount.
pub(crate) fn default_fee_strategy() -> AddressFundsFeeStrategy {
    vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]
}

/// Bank-funding fee strategy: deduct fee from input #0 so the
/// recipient receives the **exact** requested amount.
///
/// Used by [`super::bank::BankWallet::fund_address`] so
/// downstream calls — e.g. `register_identity_from_addresses(
/// {addr: N}, ...)` — don't have to compensate for fee
/// deduction at the recipient.
///
/// Tests that need the alternative `ReduceOutput(0)` semantics
/// (e.g. PA-002b verifying `Σ outputs + fee == input balance`)
/// should call [`default_fee_strategy`] explicitly.
pub(crate) fn bank_fee_strategy() -> AddressFundsFeeStrategy {
    vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]
}

/// Rebalance an explicit-input map so its sum equals `Σ outputs`.
///
/// `AddressFundsTransferTransition` validation rejects with
/// `InputOutputBalanceMismatchError` unless the encoded transition
/// satisfies `Σ inputs == Σ outputs`. With `[ReduceOutput(0)]` (the
/// harness default) the chain-time fee is taken from output 0 at
/// execution; the transition payload must still balance pre-fee.
///
/// Caller-supplied per-address values act as relative weights — a
/// single-input map is assigned the full output sum; multi-input
/// maps split the output sum proportionally with any rounding
/// remainder absorbed by the lex-smallest entry. Each share is held
/// at or above `min_input_amount` (the protocol's per-input floor) by
/// pulling the deficit from the donor with the largest share that
/// still has headroom.
fn balance_explicit_inputs(
    inputs: &BTreeMap<PlatformAddress, Credits>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    platform_version: &PlatformVersion,
) -> FrameworkResult<BTreeMap<PlatformAddress, Credits>> {
    if inputs.is_empty() {
        return Err(FrameworkError::Wallet(
            "transfer_capturing_st_bytes requires at least one input address".into(),
        ));
    }
    let total_output: Credits = outputs.values().copied().sum();
    let min_input = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;
    if total_output < min_input {
        return Err(FrameworkError::Wallet(format!(
            "Σ outputs {total_output} < min_input_amount {min_input}: cannot \
             build a balanced explicit-input map"
        )));
    }

    // Single input: assign the full output sum directly. This is the
    // PA-006 / PA-006b shape and the path that matters in practice.
    if inputs.len() == 1 {
        let addr = *inputs.keys().next().expect("len == 1");
        let mut out = BTreeMap::new();
        out.insert(addr, total_output);
        return Ok(out);
    }

    // Multi-input: weight by caller values. Zero-sum weights collapse
    // to equal share to avoid div-by-zero.
    let weight_total: u128 = inputs.values().map(|w| *w as u128).sum();
    let n = inputs.len() as u128;
    let mut shares: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut assigned: u128 = 0;
    for (addr, weight) in inputs {
        let share = if weight_total == 0 {
            (total_output as u128) / n
        } else {
            ((total_output as u128) * (*weight as u128)) / weight_total
        };
        shares.insert(*addr, share as Credits);
        assigned += share;
    }
    // Lex-smallest entry absorbs the rounding remainder so Σ matches.
    let remainder = (total_output as u128).saturating_sub(assigned) as Credits;
    if remainder > 0 {
        if let Some((_, slot)) = shares.iter_mut().next() {
            *slot = slot.saturating_add(remainder);
        }
    }

    // Lift any sub-floor share by pulling the deficit from the largest
    // peer that retains ≥ min_input after the donation.
    let needs_lift: Vec<(PlatformAddress, Credits)> = shares
        .iter()
        .filter(|(_, v)| **v < min_input)
        .map(|(a, v)| (*a, *v))
        .collect();
    for (addr, share) in needs_lift {
        let deficit = min_input - share;
        let donor = shares
            .iter()
            .filter(|(a, v)| **a != addr && **v >= min_input.saturating_add(deficit))
            .max_by_key(|(_, v)| **v)
            .map(|(a, _)| *a);
        let Some(donor) = donor else {
            return Err(FrameworkError::Wallet(format!(
                "cannot satisfy min_input_amount {min_input} on {n} inputs with \
                 Σ outputs {total_output}; no donor with sufficient headroom"
            )));
        };
        if let Some(slot) = shares.get_mut(&donor) {
            *slot -= deficit;
        }
        if let Some(slot) = shares.get_mut(&addr) {
            *slot += deficit;
        }
    }

    debug_assert_eq!(
        shares.values().copied().sum::<Credits>(),
        total_output,
        "balanced inputs must sum to Σ outputs"
    );
    Ok(shares)
}

/// Default timeout for [`TestWallet::register_identity_from_addresses`]
/// to observe the new identity on chain.
const DEFAULT_IDENTITY_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(30);

/// A registered identity returned by
/// [`TestWallet::register_identity_from_addresses`].
///
/// Bundles the on-chain identifier with the four placeholder keys
/// (MASTER + HIGH + TRANSFER + CRITICAL auth) and the seed-backed
/// identity signer so callers can drive identity-side state
/// transitions (top-up, transfer, DPNS register, token mint/burn/...)
/// without re-deriving anything.
pub struct RegisteredIdentity {
    /// On-chain identity identifier.
    pub id: Identifier,
    /// MASTER auth key (DPP `KeyID = 0`). Required for
    /// identity-mutation transitions (e.g. `IdentityUpdate`).
    pub master_key: IdentityPublicKey,
    /// HIGH auth key (DPP `KeyID = 1`). Used for `DataContractCreate`
    /// (CRITICAL or HIGH accepted) and most credit-balance state
    /// transitions.
    pub high_key: IdentityPublicKey,
    /// TRANSFER + CRITICAL key (DPP `KeyID = 2`). Required by DPP
    /// for `IdentityCreditTransferTransition` — see rs-dpp
    /// `identity_credit_transfer_transition/v0/v0_methods.rs:63-83`.
    pub transfer_key: IdentityPublicKey,
    /// AUTHENTICATION + CRITICAL key (DPP `KeyID = 3`). Required for
    /// every token-batch state transition (mint, burn, transfer,
    /// freeze, unfreeze, destroy_frozen, pause, resume, set_price,
    /// purchase, update_config). DPP's `TokenBaseTransition`
    /// `security_level_requirement` returns only
    /// `SecurityLevel::CRITICAL`; signing with HIGH yields
    /// `InvalidSignaturePublicKeySecurityLevelError` at chain
    /// validation.
    pub critical_key: IdentityPublicKey,
    /// `Arc`-shared signer pre-derived for this identity's DIP-9 slot.
    /// `Arc` lets callers hand the same signer to multiple state-transition
    /// builders without re-creating the key cache.
    pub signer: Arc<SeedBackedIdentitySigner>,
    /// DIP-9 identity index used during registration.
    pub identity_index: u32,
    /// Pre-fee credits that funded the identity at `register_from_addresses`.
    pub funding: Credits,
}

impl std::fmt::Debug for RegisteredIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredIdentity")
            .field("id", &self.id)
            .field("identity_index", &self.identity_index)
            .field("funding", &self.funding)
            .finish_non_exhaustive()
    }
}

/// Generate a fresh 64-byte seed plus its hex encoding for the
/// registry. Single source so signer + registry stay in sync.
pub fn fresh_seed() -> ([u8; 64], String) {
    let mut seed = [0u8; 64];
    OsRng.fill_bytes(&mut seed);
    let hex = hex::encode(seed);
    (seed, hex)
}

/// Build a registry entry for a fresh seed. Insert it BEFORE
/// handing the wallet to the test body so a panic between insert
/// and teardown leaves a recoverable trail.
pub fn registry_entry_from_seed(seed: &[u8; 64], note: Option<String>) -> RegistryEntry {
    RegistryEntry {
        seed_hex: hex::encode(seed),
        created_at: SystemTime::now(),
        status: EntryStatus::Active,
        note,
    }
}

/// Guard returned by [`super::setup`].
///
/// Tests SHOULD call [`SetupGuard::teardown`] explicitly once
/// they're done; the [`Drop`] impl is a panic-safety fallback that
/// logs a warning and relies on the next-startup
/// `cleanup::sweep_orphans` to recover funds.
pub struct SetupGuard {
    /// Process-shared context (`&'static` — `E2eContext::init`
    /// returns a singleton).
    pub ctx: &'static E2eContext,
    /// Fresh-seed test wallet, already registered for cleanup.
    pub test_wallet: TestWallet,
    /// Set to `true` by a successful [`SetupGuard::teardown`] so
    /// [`Drop`] skips its warning.
    pub(crate) teardown_called: bool,
}

impl SetupGuard {
    /// Sweep the test wallet's funds back to the bank and remove
    /// its registry entry.
    ///
    /// Best-effort: a transient sync / transfer failure retains the
    /// registry entry, so the next process startup retries via
    /// [`super::cleanup::sweep_orphans`].
    pub async fn teardown(mut self) -> FrameworkResult<()> {
        let result = super::cleanup::teardown_one(
            self.ctx.manager(),
            self.ctx.bank(),
            self.ctx.bank_identity(),
            self.ctx.registry(),
            &self.test_wallet,
        )
        .await;
        if result.is_ok() {
            self.teardown_called = true;
        }
        result
    }
}

impl Drop for SetupGuard {
    fn drop(&mut self) {
        if !self.teardown_called {
            tracing::warn!(
                wallet_id = %hex::encode(self.test_wallet.id()),
                "SetupGuard dropped without explicit teardown — wallet will be \
                 swept on next test process startup"
            );
        }
    }
}

/// `PlatformWalletError` → framework error envelope.
fn wallet_err(err: PlatformWalletError) -> FrameworkError {
    FrameworkError::Wallet(err.to_string())
}

/// Generate the address at DIP-17 slot-0 of (account=0, key_class=0)
/// and mark it used in the address pool, so the next call to
/// `next_unused_receive_address` returns slot-1 instead.
///
/// QA-002 fix: shifts every test wallet's "first usable address" off
/// the slot reserved for [`super::bank::BankWallet::primary_receive_address`].
async fn consume_platform_address_index_zero(wallet: &Arc<PlatformWallet>) -> FrameworkResult<()> {
    // Generates index 0 with `add_to_state=true`, populating the pool
    // so `mark_index_used(0)` can find it below.
    let _index_zero = wallet
        .platform()
        .next_unused_receive_address(default_platform_payment_account_key())
        .await
        .map_err(wallet_err)?;

    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let info = wm.get_wallet_info_mut(&wallet_id).ok_or_else(|| {
        FrameworkError::Wallet(format!(
            "wallet {} missing from manager during slot-0 consume",
            hex::encode(wallet_id)
        ))
    })?;
    let account = info
        .core_wallet
        .platform_payment_managed_account_at_index_mut(DEFAULT_ACCOUNT_INDEX_PUB)
        .ok_or_else(|| {
            FrameworkError::Wallet(format!(
                "no platform-payment account at index {DEFAULT_ACCOUNT_INDEX_PUB} \
                 during slot-0 consume"
            ))
        })?;
    if !account.addresses.mark_index_used(0) {
        return Err(FrameworkError::Wallet(
            "mark_index_used(0) returned false: slot-0 missing from pool \
             or already marked used"
                .into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drift guard: our pinned defaults must match `PlatformPaymentAccountSpec::default()`.
    /// If `key_wallet` ever changes its canonical defaults, this test fires.
    #[test]
    fn default_spec_matches_pinned_constants() {
        let canonical = PlatformPaymentAccountSpec::default();
        assert_eq!(canonical.account, DEFAULT_ACCOUNT_INDEX_PUB);
        assert_eq!(canonical.key_class, DEFAULT_KEY_CLASS_PUB);
        assert_eq!(canonical, DEFAULT_PLATFORM_PAYMENT_ACCOUNT_SPEC);
    }

    fn addr(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    /// PA-006 / PA-006b shape: one input address, one output address.
    /// Caller passes the address's full balance as the input amount;
    /// the helper must rewrite it to `Σ outputs` so the protocol's
    /// `Σ in == Σ out` check passes.
    #[test]
    fn balance_explicit_inputs_single_address_matches_output_sum() {
        let pv = PlatformVersion::latest();
        let in_addr = addr(0x01);
        let out_addr = addr(0x02);
        let inputs: BTreeMap<_, _> = std::iter::once((in_addr, 90_755_960u64)).collect();
        let outputs: BTreeMap<_, _> = std::iter::once((out_addr, 50_000_000u64)).collect();

        let balanced = balance_explicit_inputs(&inputs, &outputs, pv).expect("balance");
        assert_eq!(balanced.len(), 1);
        assert_eq!(balanced.get(&in_addr).copied(), Some(50_000_000));
        let in_sum: Credits = balanced.values().copied().sum();
        let out_sum: Credits = outputs.values().copied().sum();
        assert_eq!(in_sum, out_sum, "Σ inputs must equal Σ outputs");
    }

    /// Multi-input shape: split `Σ outputs` proportionally to the
    /// caller-supplied weights; sum must match exactly.
    #[test]
    fn balance_explicit_inputs_multi_address_sum_matches() {
        let pv = PlatformVersion::latest();
        let a = addr(0x01);
        let b = addr(0x02);
        let out = addr(0x09);
        let inputs: BTreeMap<_, _> = [(a, 30_000_000u64), (b, 70_000_000u64)]
            .into_iter()
            .collect();
        let outputs: BTreeMap<_, _> = std::iter::once((out, 50_000_001u64)).collect();

        let balanced = balance_explicit_inputs(&inputs, &outputs, pv).expect("balance");
        assert_eq!(balanced.len(), 2);
        let in_sum: Credits = balanced.values().copied().sum();
        assert_eq!(in_sum, 50_000_001, "Σ inputs must equal Σ outputs exactly");

        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        for (a, v) in &balanced {
            assert!(
                *v >= min_input,
                "share for {a:?} = {v} below min_input {min_input}"
            );
        }
    }

    /// Empty inputs are rejected up-front; the protocol requires ≥ 1
    /// input on every transfer transition.
    #[test]
    fn balance_explicit_inputs_rejects_empty() {
        let pv = PlatformVersion::latest();
        let outputs: BTreeMap<_, _> = std::iter::once((addr(0x09), 50_000_000u64)).collect();
        let err = balance_explicit_inputs(&BTreeMap::new(), &outputs, pv).unwrap_err();
        assert!(matches!(err, FrameworkError::Wallet(_)));
    }
}
