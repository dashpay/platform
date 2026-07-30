//! **Funding-domain isolation** — the privacy invariant every L1 spend path in
//! this crate must satisfy, plus the automated guardrail that enforces it.
//!
//! # The invariant
//!
//! > A single L1 transaction must draw its funding inputs from **exactly one**
//! > funds account (one derivation domain). No spend path may union across
//! > funding accounts, and never implicitly.
//!
//! A wallet's funds accounts are separate *privacy domains*: ordinary BIP44 /
//! BIP32 coins, DIP-9 **CoinJoin** (previously-mixed) coins, and
//! **DashPay-receiving** coins each carry a different linkability story. Any
//! transaction that spends inputs from two of them publishes, irreversibly and
//! on chain, that the same entity controls both — and shielding those coins
//! afterwards cannot undo the link, because the link is already in the L1
//! transaction graph.
//!
//! # History — why this module exists
//!
//! * **dashpay/platform#4073** — shielding failed on wallets whose balance sat
//!   on the DIP-9 CoinJoin path, because asset-lock coin selection only ever
//!   reached BIP44 account 0.
//! * The **first** fix unioned every funding account and ran `LargestFirst`
//!   over the combined candidate set. Reviewer `shumkov` **blocked** it on
//!   dashpay/platform#4184 (2026-07-21): largest-first over a union can combine
//!   ordinary, CoinJoin, and DashPay-receiving coins into one transaction with
//!   BIP44 change, irreversibly linking the domains.
//! * **dashpay/platform#4184** (commit `4d3e1322bc`, signed off 2026-07-23) is
//!   the approved resolution: a single optional derivation-path parameter that
//!   names the ONE funding account, **defaulting to the non-mixed BIP44
//!   account**. No union, and no consent gate — because the caller names
//!   exactly one funding source, there is nothing to consent to.
//!
//! The regression this module guards against is real and already happened once:
//! `CoreWallet::build_signed_payment` (dashpay/platform#4247) was written
//! against the pre-re-scope design and shipped the blocked union behavior into
//! the general send path.
//!
//! # How to comply
//!
//! A spend path takes `funding_path: Option<DerivationPath>`, resolves `None`
//! to the unmixed BIP44 account's account-level path, locates the **one**
//! managed funds account whose account-level path equals it, and points the
//! key-wallet `TransactionBuilder`'s `set_funding` at that account alone. If
//! that account cannot cover the spend, the build **fails** with a typed
//! shortfall — it must never top up from a second account. See
//! [`CoreWallet::build_signed_payment`](crate::wallet::core::CoreWallet::build_signed_payment)
//! for the reference implementation on this branch.
//!
//! Change is the one structural exception, and it is not a co-spend: key-wallet
//! derives change addresses only for *Standard* (BIP44/BIP32) accounts, so a
//! transaction funded from a non-Standard account (CoinJoin / DashPay
//! receiving) must route its change to the BIP44 account. That is inherent to
//! spending those coins at all, happens only when the caller explicitly named
//! the non-default account, and is the behavior #4184 approved.
//!
//! # The guardrail
//!
//! `all_funding_accounts()` / `all_funding_accounts_mut()` live in the pinned
//! `key-wallet` fork, so the invariant cannot be documented at their
//! definition. Instead, [`guardrail::every_union_iteration_is_privacy_reviewed`]
//! scans this crate's own sources and fails if any use of those iterators is
//! not preceded by an explicit `PRIVACY-DOMAIN-OK:` review marker, and
//! [`guardrail::no_spend_entry_point_unions_by_default`] asserts behaviorally
//! that the send path does not cross domains. See the `guardrail` module docs
//! for why the pair is sufficient.

use key_wallet::ManagedAccountType;

/// Whether a funds account can be *signed for* by the local mnemonic, and may
/// therefore fund a spend at all.
///
/// Only `DashpayExternalAccount`s are watch-only: they hold a **contact's**
/// receiving addresses (we keep the contact's xpub to build payments *to* them
/// and to watch that side), so no private key of ours derives their UTXOs and
/// signing them would fail. Every other funds account
/// (BIP44 / BIP32 / CoinJoin / DashPay-receiving) comes from our own seed.
///
/// This is a **signability** filter, not a privacy filter — it is orthogonal to
/// the module-level funding-domain invariant, and passing it does NOT make an
/// account eligible to be unioned with another. A watch-only account must be
/// refused even when a caller names its derivation path explicitly.
pub(crate) fn is_signable_funding_account(managed_type: &ManagedAccountType) -> bool {
    !matches!(
        managed_type,
        ManagedAccountType::DashpayExternalAccount { .. }
    )
}

#[cfg(test)]
mod guardrail {
    //! Automated enforcement of the funding-domain invariant.
    //!
    //! Two complementary tests, because either alone has a blind spot:
    //!
    //! * [`no_spend_entry_point_unions_by_default`] is **behavioral**. It
    //!   proves, over a wallet whose balance is split across two domains, that
    //!   the general send entry point cannot fund a transaction neither domain
    //!   covers alone. This catches a semantic regression in an *existing*
    //!   entry point even if it is written without ever naming
    //!   `all_funding_accounts` (e.g. by iterating the account maps directly).
    //!   Its blind spot: it only knows about the entry points listed in it, so
    //!   a *newly added* spend path is invisible to it.
    //!
    //!   On this branch (the isolated `send-raw-tx` feature line) the only
    //!   funding-domain-sensitive spend entry point is
    //!   [`CoreWallet::build_signed_payment`]: the asset-lock builder here is
    //!   still the pre-#4184 per-`account_index` model and does not accept a
    //!   `funding_path`, so it is out of scope for this behavioral test.
    //!
    //! * [`every_union_iteration_is_privacy_reviewed`] is **static**, and
    //!   covers exactly that blind spot. `all_funding_accounts()` /
    //!   `all_funding_accounts_mut()` are the only wallet-wide funds-account
    //!   iterators key-wallet exposes, so a new unparameterized union spend
    //!   path essentially has to call one of them. This test fails unless each
    //!   such call is preceded by an explicit `PRIVACY-DOMAIN-OK:` marker
    //!   comment, which forces the author to state why the call does not
    //!   union — and makes the marker show up in the review diff, which is how
    //!   the #4247 regression should have been caught.
    //!
    //! Scope is this crate's `src/` on purpose: coin selection happens only
    //! here. The FFI / JNI layers above merely forward a `funding_path` and
    //! cannot select coins themselves.

    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use dashcore::{Address as DashAddress, Network, OutPoint};

    use crate::test_support::{split_funded_wallet_manager, AlwaysRejectedBroadcaster};
    use crate::wallet::core::balance::WalletBalance;
    use crate::wallet::core::CoreWallet;
    use crate::PlatformWalletError;

    // -- static guard --------------------------------------------------------

    /// The wallet-wide funds-account iterators. Any use of these in a spend
    /// path is a potential cross-domain union.
    const UNION_ITERATORS: [&str; 2] = ["all_funding_accounts(", "all_funding_accounts_mut("];

    /// The marker a call site must carry to certify it was reviewed against the
    /// funding-domain invariant.
    const REVIEW_MARKER: &str = "PRIVACY-DOMAIN-OK";

    /// How many lines above a call site the marker may sit. Wide enough for the
    /// explanatory comment block a legitimate use needs, narrow enough that one
    /// marker cannot silently cover an unrelated call added later.
    const MARKER_LOOKBACK_LINES: usize = 15;

    /// Collect every `.rs` file under `dir`, recursively.
    fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("readable dir entry").path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Every use of key-wallet's wallet-wide funds-account iterators must be
    /// explicitly privacy-reviewed.
    ///
    /// Fails with the offending `file:line` and the invariant restated, so an
    /// author who reintroduces an unparameterized `all_funding_accounts()`
    /// spend path (the dashpay/platform#4247 regression) is told exactly what
    /// rule they tripped and where the contract lives.
    ///
    /// Comment lines are ignored (prose may name the iterators freely), as is
    /// this file — it names them in the constants above.
    #[test]
    fn every_union_iteration_is_privacy_reviewed() {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rust_sources(&src, &mut files);
        assert!(
            !files.is_empty(),
            "found no Rust sources under {} — the guardrail would silently pass",
            src.display()
        );

        let this_file = Path::new(file!())
            .file_name()
            .expect("this file has a name")
            .to_owned();

        let mut unreviewed = Vec::new();
        for file in &files {
            if file.file_name() == Some(this_file.as_os_str()) {
                continue;
            }
            let text = std::fs::read_to_string(file)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                // Prose is free to discuss the iterators.
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if !UNION_ITERATORS.iter().any(|needle| line.contains(needle)) {
                    continue;
                }
                let from = i.saturating_sub(MARKER_LOOKBACK_LINES);
                let reviewed = lines[from..=i].iter().any(|l| l.contains(REVIEW_MARKER));
                if !reviewed {
                    unreviewed.push(format!(
                        "  {}:{}: {}",
                        file.strip_prefix(&src).unwrap_or(file).display(),
                        i + 1,
                        line.trim()
                    ));
                }
            }
        }

        assert!(
            unreviewed.is_empty(),
            "FUNDING-DOMAIN INVARIANT: unreviewed use of key-wallet's wallet-wide \
             funds-account iterator(s):\n{}\n\n\
             A single L1 transaction must draw its inputs from EXACTLY ONE funds \
             account. Unioning ordinary BIP44/BIP32, CoinJoin, and DashPay-receiving \
             coins into one transaction irreversibly links those privacy domains on \
             chain (dashpay/platform#4073, blocked and re-scoped by #4184; regressed \
             once already in #4247).\n\n\
             If your call site does NOT select coins across accounts (e.g. it is \
             looking one account up by derivation path, or reading balances), add a \
             comment containing `{REVIEW_MARKER}:` within {MARKER_LOOKBACK_LINES} \
             lines above it saying why. If it DOES select across accounts, it is the \
             bug — take a `funding_path: Option<DerivationPath>` instead and fund \
             from the one named account, defaulting to unmixed BIP44. See \
             `wallet::funding_privacy`.",
            unreviewed.join("\n"),
        );
    }

    // -- behavioral guard ----------------------------------------------------

    /// Duffs on BIP44 account 0 in the split fixture.
    const BIP44_DUFFS: u64 = 9_000_000;
    /// Duffs on DIP-9 CoinJoin account 0 in the split fixture.
    const COINJOIN_DUFFS: u64 = 9_000_000;
    /// More than either domain holds, less than their sum — fundable ONLY by a
    /// cross-domain union. The send entry point must refuse it.
    const CROSS_DOMAIN_ONLY: u64 = 15_000_000;

    /// The general L1 send entry point may not fund a transaction that requires
    /// coins from more than one funding account.
    ///
    /// The fixture splits the balance evenly across two privacy domains (BIP44
    /// and DIP-9 CoinJoin) and asks the default (unmixed BIP44) send path for an
    /// amount that neither domain covers alone but their union does. A
    /// union-funding path succeeds here; a compliant single-domain path fails.
    /// Failing is the CORRECT outcome: selection is confined to the named
    /// account, and a shortfall surfaces as a typed insufficient-funds error
    /// rather than silently reaching into a second domain.
    #[tokio::test]
    async fn no_spend_entry_point_unions_by_default() {
        let (wm, wallet_id, signer) =
            split_funded_wallet_manager(BIP44_DUFFS, COINJOIN_DUFFS).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            wm,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::new(WalletBalance::new()),
        );
        let payment = core
            .build_signed_payment(
                vec![(DashAddress::dummy(Network::Testnet, 7), CROSS_DOMAIN_ONLY)],
                None,
                &signer,
                None,
            )
            .await;
        assert!(
            matches!(
                payment,
                Err(PlatformWalletError::PaymentInsufficientFunds { .. })
            ),
            "build_signed_payment must not union BIP44 with CoinJoin, got {payment:?}"
        );
    }

    /// The flip side of the invariant: naming a domain explicitly confines
    /// selection to it and to nothing else. Asks for an amount BIP44 alone
    /// could not cover, from a CoinJoin account that can — and asserts no BIP44
    /// input rides along.
    #[tokio::test]
    async fn an_explicit_domain_selects_strictly_within_itself() {
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

        // 0.09 DASH BIP44, 0.2 DASH CoinJoin; take 0.15 DASH from CoinJoin.
        let (wm, wallet_id, signer) = split_funded_wallet_manager(9_000_000, 20_000_000).await;

        let (bip44_ops, coinjoin_ops, coinjoin_path) = {
            let guard = wm.read().await;
            let (_, info) = guard.get_wallet_and_info(&wallet_id).expect("wallet present");
            let network = info.core_wallet.network();
            let bip44: HashSet<OutPoint> = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .map(|a| a.utxos.keys().copied().collect())
                .unwrap_or_default();
            let coinjoin_acc = info
                .core_wallet
                .accounts
                .coinjoin_accounts
                .get(&0)
                .expect("coinjoin account 0 present");
            let coinjoin: HashSet<OutPoint> = coinjoin_acc.utxos.keys().copied().collect();
            let path = coinjoin_acc
                .managed_account_type()
                .to_account_type()
                .derivation_path(network)
                .expect("coinjoin account-level path");
            (bip44, coinjoin, path)
        };

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let core = CoreWallet::new(
            sdk,
            wm,
            wallet_id,
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::new(WalletBalance::new()),
        );
        let payment = core
            .build_signed_payment(
                vec![(DashAddress::dummy(Network::Testnet, 7), CROSS_DOMAIN_ONLY)],
                None,
                &signer,
                Some(coinjoin_path),
            )
            .await
            .expect("the named CoinJoin account covers 0.15 DASH");

        let spent: HashSet<OutPoint> = payment
            .transaction
            .input
            .iter()
            .map(|i| i.previous_output)
            .collect();
        assert!(
            spent.iter().all(|op| coinjoin_ops.contains(op)),
            "every input must come from the named CoinJoin account, spent {spent:?}"
        );
        assert!(
            !spent.iter().any(|op| bip44_ops.contains(op)),
            "an explicitly-named domain must not pull BIP44 inputs, spent {spent:?}"
        );
    }
}
