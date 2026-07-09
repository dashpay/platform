use std::collections::BTreeMap;

use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use dpp::version::PlatformVersion;
use key_wallet::PlatformP2PKHAddress;

use crate::error::promote_address_nonce_error_or_sdk;
use crate::wallet::PlatformAddressWallet;
use crate::{PlatformAddressChangeSet, PlatformWalletError};
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;

pub use super::InputSelection;
use super::{checked_sum_credits, saturating_sum_credits};

/// Address-keyed step in a fee strategy. Resolves to an
/// [`AddressFundsFeeStrategyStep`] by looking up the named address in the
/// final inputs / outputs maps that the signer will see.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeeStrategyStepByAddress {
    /// Deduct fee from the named input address's remaining balance.
    DeductFromInputAddress(PlatformAddress),
    /// Reduce the named output address's credited amount by the fee.
    ReduceOutputAddress(PlatformAddress),
}

/// Address-keyed analogue of [`AddressFundsFeeStrategy`].
///
/// Used by [`PlatformAddressWallet::transfer_with_change_address`] so
/// callers identify the fee-bearing row by address rather than by index
/// into the post-canonicalisation `BTreeMap`. Lowered to the consensus
/// [`AddressFundsFeeStrategy`] inside the wrapper, AFTER augmentation,
/// when the final outputs map is known.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeeStrategyByAddress(pub Vec<FeeStrategyStepByAddress>);

/// Errors specific to lowering [`FeeStrategyByAddress`] to
/// [`AddressFundsFeeStrategy`]. Folds into
/// [`PlatformWalletError::AddressOperation`].
#[derive(Debug, thiserror::Error)]
pub enum FeeStrategyResolveError {
    #[error("DeductFromInputAddress: address {0:?} not present in inputs map")]
    InputAddressNotFound(PlatformAddress),
    #[error("ReduceOutputAddress: address {0:?} not present in outputs map")]
    OutputAddressNotFound(PlatformAddress),
    #[error("{kind} index {index} exceeds u16::MAX (BTreeMap holds > 65535 entries)")]
    IndexOverflow { kind: &'static str, index: usize },
}

// INTENTIONAL(CMT-004): FeeStrategyResolveError variants flatten into
// PlatformWalletError::AddressOperation(String) rather than being promoted
// to typed PlatformWalletError variants. Rationale: fee-strategy resolution
// errors are caller-input errors (caller named an address not in the
// inputs/outputs map they themselves supplied) distinct from wallet-state
// errors like ChangeBelowMinimumOutput / InputSumOverflow /
// OnlyOutputAddressesFunded. Keeping the wallet error enum lean by
// flattening caller-input mistakes into a string is the deliberate trade-off.
impl From<FeeStrategyResolveError> for PlatformWalletError {
    fn from(e: FeeStrategyResolveError) -> Self {
        PlatformWalletError::AddressOperation(e.to_string())
    }
}

impl FeeStrategyByAddress {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Convenience: target an input address for fee deduction.
    pub fn deduct_from_input(addr: PlatformAddress) -> Self {
        Self(vec![FeeStrategyStepByAddress::DeductFromInputAddress(addr)])
    }

    /// Convenience: target an output address (typically the change output)
    /// for fee reduction.
    pub fn reduce_output(addr: PlatformAddress) -> Self {
        Self(vec![FeeStrategyStepByAddress::ReduceOutputAddress(addr)])
    }

    /// Lower to consensus-indexed [`AddressFundsFeeStrategy`].
    ///
    /// `inputs` and `outputs` MUST be the FINAL maps the signer will use —
    /// i.e. for outputs, the post-`augment_outputs_with_change` map
    /// containing the change row when applicable. Calling this before
    /// augmentation will produce incorrect indexes.
    pub fn to_indexed<V>(
        &self,
        inputs: &BTreeMap<PlatformAddress, V>,
        outputs: &BTreeMap<PlatformAddress, Credits>,
    ) -> Result<AddressFundsFeeStrategy, FeeStrategyResolveError> {
        self.0
            .iter()
            .map(|step| step.to_indexed(inputs, outputs))
            .collect()
    }
}

impl FeeStrategyStepByAddress {
    pub fn to_indexed<V>(
        &self,
        inputs: &BTreeMap<PlatformAddress, V>,
        outputs: &BTreeMap<PlatformAddress, Credits>,
    ) -> Result<AddressFundsFeeStrategyStep, FeeStrategyResolveError> {
        match self {
            FeeStrategyStepByAddress::DeductFromInputAddress(addr) => {
                let pos = inputs
                    .keys()
                    .position(|k| k == addr)
                    .ok_or(FeeStrategyResolveError::InputAddressNotFound(*addr))?;
                let idx =
                    u16::try_from(pos).map_err(|_| FeeStrategyResolveError::IndexOverflow {
                        kind: "input",
                        index: pos,
                    })?;
                Ok(AddressFundsFeeStrategyStep::DeductFromInput(idx))
            }
            FeeStrategyStepByAddress::ReduceOutputAddress(addr) => {
                let pos = outputs
                    .keys()
                    .position(|k| k == addr)
                    .ok_or(FeeStrategyResolveError::OutputAddressNotFound(*addr))?;
                let idx =
                    u16::try_from(pos).map_err(|_| FeeStrategyResolveError::IndexOverflow {
                        kind: "output",
                        index: pos,
                    })?;
                Ok(AddressFundsFeeStrategyStep::ReduceOutput(idx))
            }
        }
    }
}

impl PlatformAddressWallet {
    /// Transfer credits between platform addresses.
    ///
    /// Input addresses can be specified explicitly or selected automatically
    /// from the account via [`InputSelection::Auto`]. When `platform_version`
    /// is `None`, the wallet's SDK version (`self.sdk.version()`) drives fee
    /// estimation and every version-keyed limit (`min_input_amount`,
    /// `max_address_inputs`) during auto-selection — the same source the UI
    /// preflight reads, so the submit gate and the spend path never diverge on
    /// a non-latest-pinned SDK. An explicit `Some(v)` is honored as given.
    ///
    /// `address_signer` produces ECDSA signatures for the input
    /// [`PlatformAddress`]es; the wallet itself holds no key material —
    /// callers supply a seed-backed, hardware, or FFI-trampoline signer.
    ///
    /// # When to use this vs `transfer_with_change_address`
    ///
    /// - `transfer()` — simple entry point with an **indexed**
    ///   [`AddressFundsFeeStrategy`]. Supports all three [`InputSelection`]
    ///   variants. Use when you can supply fee-strategy indices directly
    ///   against the canonical lex-sorted outputs map, or when you want
    ///   auto-selection.
    /// - [`Self::transfer_with_change_address`] — wrapper that takes an
    ///   **address-keyed** [`FeeStrategyByAddress`] and optionally routes
    ///   surplus from explicit inputs to a change output. Use when you want
    ///   to identify the fee-bearing row by address (the wrapper resolves to
    ///   indices after change-augmentation) and/or want the wallet to compute
    ///   the change amount from over-funded explicit inputs. Rejects
    ///   [`InputSelection::Auto`].
    ///
    /// # Invariant: `Σ inputs == Σ outputs`
    ///
    /// The protocol enforces strict equality. Responsibility for satisfying
    /// it depends on `input_selection`:
    ///
    /// - [`InputSelection::Auto`]: the wallet trims selected inputs to match
    ///   the output sum. No work for the caller. Supported fee strategies
    ///   under Auto: `[DeductFromInput(0)]` and `[ReduceOutput(0)]` only.
    /// - [`InputSelection::Explicit`] / [`InputSelection::ExplicitWithNonces`]:
    ///   **caller** must construct the inputs map such that the sum equals
    ///   `Σ outputs`. For automatic surplus routing to a change output, use
    ///   [`Self::transfer_with_change_address`] with
    ///   `output_change_address: Some(_)`.
    ///
    /// # Output order semantics
    ///
    /// Outputs are stored on-chain in `AddressFundsTransferTransitionV0` as a
    /// `BTreeMap<PlatformAddress, Credits>` — keyed by address, iterated in
    /// lexicographic order. This is also the index space resolved by the
    /// fee-strategy `ReduceOutput(i)`: index 0 is the lex-smallest output
    /// address, not the first one supplied by the caller. The parameter type
    /// `BTreeMap` is chosen to make this canonical ordering explicit at the
    /// type signature; duplicate destination addresses are structurally
    /// impossible.
    pub async fn transfer<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        outputs: BTreeMap<PlatformAddress, Credits>,
        fee_strategy: AddressFundsFeeStrategy,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if outputs.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "Transfer requires at least one output address".to_string(),
            ));
        }

        // Single source of truth for the planning version: when the caller
        // pins an explicit `Some(v)` we honor it, but the default is the
        // wallet's SDK version (`self.sdk.version()`) — NOT
        // `LATEST_PLATFORM_VERSION`. This is the same network-floored,
        // protocol-version-tracking accessor that the UI preflight and the
        // `min_input_amount` / `min_output_amount` getters read, so the submit
        // gate and this spend path size every version-keyed value
        // (`min_input_amount`, `max_address_inputs`, and `estimate_min_fee`)
        // against the SAME version. Defaulting to LATEST here would let the
        // gate and the spend path diverge on a non-latest-pinned SDK.
        let version = platform_version.unwrap_or_else(|| self.sdk.version());

        // `proof_height` is the broadcast proof's committed block — the
        // height pin for the reconciled absolutes below. Transfer outputs
        // are recorded on-chain as `AddBalanceToAddress` DELTAS at that
        // height (a same-wallet output — e.g. a change address — is the
        // ADDR-09 double-count shape); the pin is what stops the sync from
        // re-applying them. See `reconcile_address_infos`.
        let (address_infos, proof_height) = match input_selection {
            InputSelection::Explicit(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Transfer requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .transfer_address_funds(inputs, outputs, fee_strategy, address_signer, None)
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
            }
            InputSelection::ExplicitWithNonces(inputs) => {
                if inputs.is_empty() {
                    return Err(PlatformWalletError::AddressOperation(
                        "Transfer requires at least one input address".to_string(),
                    ));
                }
                self.sdk
                    .transfer_address_funds_with_nonce(
                        inputs,
                        outputs,
                        fee_strategy,
                        address_signer,
                        None,
                    )
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
            }
            InputSelection::Auto => {
                // Auto-select supports `[DeductFromInput(0)]` and `[ReduceOutput(0)]`;
                // any other shape must use `Explicit`.
                if !matches!(
                    fee_strategy.as_slice(),
                    [AddressFundsFeeStrategyStep::DeductFromInput(0)]
                        | [AddressFundsFeeStrategyStep::ReduceOutput(0)]
                ) {
                    return Err(PlatformWalletError::AddressOperation(
                        "InputSelection::Auto supports fee_strategy = [DeductFromInput(0)] \
                         or [ReduceOutput(0)]; for other strategies use InputSelection::Explicit"
                            .to_string(),
                    ));
                }
                let inputs = self
                    .auto_select_inputs(account_index, &outputs, &fee_strategy, version)
                    .await?;
                self.sdk
                    .transfer_address_funds(inputs, outputs, fee_strategy, address_signer, None)
                    .await
                    .map_err(promote_address_nonce_error_or_sdk)?
            }
        };

        // `transfer_address_funds` returns address info for the full
        // `inputs ∪ outputs` set, including external recipients the wallet
        // does not own — the shared seam filters those out, applies the
        // proof-attested balances pinned at `proof_height`, updates the
        // sync seed, and persists. The pin lets the next BLAST pass drop
        // the on-chain credit delta instead of re-applying it on top of
        // the absolute seed (ADDR-09).
        Ok(self
            .reconcile_address_infos(&address_infos, proof_height, "address transfer")
            .await)
    }

    /// Transfer credits with an address-keyed fee strategy and an optional
    /// "change address" override.
    ///
    /// # When to use
    ///
    /// Use this wrapper when you want to identify the fee-bearing row by
    /// [`PlatformAddress`] and/or want the wallet to compute a change output
    /// from over-funded explicit inputs. For [`InputSelection::Auto`] or
    /// index-based fee strategies, use [`Self::transfer`] directly — this
    /// wrapper rejects `Auto` because the auto-selector leaves no surplus
    /// to route as change.
    ///
    /// The fee-bearing row is identified by [`PlatformAddress`] rather than
    /// by index — the wrapper lowers [`FeeStrategyByAddress`] to the
    /// consensus [`AddressFundsFeeStrategy`] AFTER `user_outputs` has been
    /// augmented with the change row, so indexes always resolve against the
    /// final lex-ordered outputs map the signer will see. This eliminates a
    /// class of misrouting bugs where inserting a change address shifts the
    /// post-canonicalisation index of one or more user outputs.
    ///
    /// When `output_change_address` is `Some(change_addr)`, the wrapper adds
    /// a change output absorbing `Σ consumed − Σ user_outputs` so the
    /// `Σ inputs == Σ outputs` invariant holds. When `None`, the user
    /// outputs are forwarded as-is.
    ///
    /// Requires [`InputSelection::Explicit`] or
    /// [`InputSelection::ExplicitWithNonces`] — auto-selection trims inputs
    /// to a covering prefix (no residual to route as change) and
    /// address-keyed lowering needs the inputs map known up front. Callers
    /// who want auto-selection should use [`Self::transfer`] directly.
    ///
    /// Under `DeductFromInputAddress(_)` the caller MUST reserve fee
    /// headroom on the targeted input (i.e. its map value must be strictly
    /// below its on-chain balance by at least the estimated fee); otherwise
    /// the chain rejects the transition with `fee_fully_covered = false`.
    /// Under `ReduceOutputAddress(_)` callers may pass the full balances;
    /// the named output absorbs the fee.
    ///
    /// # Errors
    ///
    /// [`PlatformWalletError::AddressOperation`] when:
    /// - `fee_strategy` is empty,
    /// - [`InputSelection::Auto`] is supplied,
    /// - `change_addr` collides with `user_outputs` or inputs,
    /// - `Σ inputs ≤ Σ user_outputs` (no surplus for change),
    /// - a fee-strategy step names an address not present in the resolved
    ///   inputs / outputs maps.
    #[allow(clippy::too_many_arguments)] // mirrors `transfer` plus the change-address override.
    pub async fn transfer_with_change_address<S: Signer<PlatformAddress> + Send + Sync>(
        &self,
        account_index: u32,
        input_selection: InputSelection,
        user_outputs: BTreeMap<PlatformAddress, Credits>,
        output_change_address: Option<PlatformAddress>,
        fee_strategy: FeeStrategyByAddress,
        platform_version: Option<&PlatformVersion>,
        address_signer: &S,
    ) -> Result<PlatformAddressChangeSet, PlatformWalletError> {
        if fee_strategy.0.is_empty() {
            return Err(PlatformWalletError::AddressOperation(
                "fee_strategy must contain at least one step".to_string(),
            ));
        }

        // Auto is incompatible with this wrapper: the auto-selector trims
        // inputs to satisfy `Σ inputs == Σ outputs`, leaving no residual to
        // route to a change address; address-keyed fee resolution also
        // needs the inputs map known up-front.
        let inputs_for_resolve: BTreeMap<PlatformAddress, Credits> = match &input_selection {
            InputSelection::Explicit(inputs) => inputs.clone(),
            InputSelection::ExplicitWithNonces(inputs) => inputs
                .iter()
                .map(|(addr, (_nonce, credits))| (*addr, *credits))
                .collect(),
            InputSelection::Auto => {
                return Err(PlatformWalletError::AddressOperation(
                    "transfer_with_change_address requires InputSelection::Explicit or \
                     ExplicitWithNonces — the auto-selector trims inputs to a covering \
                     prefix and has no concept of a residual to route to a change address; \
                     address-keyed fee strategy also needs the inputs map known up-front. \
                     Use `transfer` for Auto selection."
                        .to_string(),
                ));
            }
        };

        // Default to the wallet's SDK version (`self.sdk.version()`) — the
        // same network-floored, protocol-version-tracking accessor that
        // `transfer()` uses — rather than `LATEST_PLATFORM_VERSION`. This
        // wrapper rejects `InputSelection::Auto`, so the production Auto UI
        // never reaches it, but defaulting to LATEST here would still let the
        // change-augmentation / fee-headroom math size version-keyed values
        // (`min_output_amount`, `estimate_min_fee`) against a different
        // version than the submit gate on a non-latest-pinned SDK. Defending
        // in depth keeps both `transfer` entry points sizing every
        // version-keyed value against the SAME version. An explicit `Some(v)`
        // is honored as given.
        let version = platform_version.unwrap_or_else(|| self.sdk.version());

        let final_outputs = match output_change_address {
            Some(change_addr) => {
                validate_change_address(&change_addr, &user_outputs, inputs_for_resolve.keys())?;
                let input_sum = checked_sum_credits(inputs_for_resolve.values().copied())?;
                augment_outputs_with_change(user_outputs, change_addr, input_sum, version)?
            }
            None => user_outputs,
        };

        // Lower fee_strategy AFTER augmentation so indexes resolve against
        // the FINAL outputs map. Lowering before would reintroduce the
        // misrouting bug this wrapper exists to prevent.
        let indexed_fee_strategy = fee_strategy.to_indexed(&inputs_for_resolve, &final_outputs)?;

        // Replicate the Auto path's ReduceOutput fee-headroom guard so callers
        // get a typed wallet-side rejection (and 3x safety-band warning)
        // instead of a generic chain-time `fee_fully_covered = false`. Covers
        // ALL `ReduceOutput(i)` steps since `to_indexed` can produce non-zero
        // indices; mirrors the Auto path's choice to guard outputs only.
        for step in indexed_fee_strategy.iter() {
            if let AddressFundsFeeStrategyStep::ReduceOutput(idx) = step {
                let target = final_outputs
                    .values()
                    .nth(*idx as usize)
                    .copied()
                    .unwrap_or(0);
                let estimated_fee = AddressFundsTransferTransition::estimate_min_fee(
                    inputs_for_resolve.len(),
                    final_outputs.len(),
                    version,
                );
                if target < estimated_fee {
                    return Err(PlatformWalletError::AddressOperation(format!(
                        "ReduceOutput target at index {} ({} credits) cannot absorb \
                         estimated fee ({} credits); raise that output or switch to \
                         DeductFromInput-based fee strategy",
                        idx, target, estimated_fee,
                    )));
                }
                const REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE: Credits = 3;
                let safe_threshold =
                    estimated_fee.saturating_mul(REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE);
                if target < safe_threshold {
                    tracing::warn!(
                        target_index = *idx,
                        target_amount = target,
                        estimated_fee,
                        safety_multiple = REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
                        tracking_issue = "platform#3040",
                        "[ReduceOutputAddress] target ({} credits) is within {}x of the \
                         static estimated fee ({} credits); chain-time fee may exceed the \
                         static estimate (platform#3040), risking on-chain rejection. \
                         Consider raising the target output or switching to \
                         DeductFromInputAddress.",
                        target,
                        REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
                        estimated_fee,
                    );
                }
            }
        }

        self.transfer(
            account_index,
            input_selection,
            final_outputs,
            indexed_fee_strategy,
            platform_version,
            address_signer,
        )
        .await
    }

    /// Dispatch to the strategy-specific selector. Returned map values are the
    /// **consumed amount per address**; protocol enforces `Σ inputs == Σ outputs`.
    /// Supported strategies: `[DeductFromInput(0)]`, `[ReduceOutput(0)]`.
    async fn auto_select_inputs(
        &self,
        account_index: u32,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
        let total_output: Credits = saturating_sum_credits(outputs.values().copied());

        // TODO(QA-007): pre-existing read-snapshot vs broadcast race; new
        // headroom math leans harder on snapshot. Hold guard across
        // snapshot+broadcast+update, or re-validate headroom right before
        // broadcast.
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "Wallet {:?} not found in wallet manager",
                hex::encode(self.wallet_id)
            ))
        })?;

        let account = info
            .core_wallet
            .platform_payment_managed_account_at_index(account_index)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account at index {}",
                    account_index
                ))
            })?;

        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;

        let address_balances: Vec<(PlatformAddress, Credits)> = account
            .addresses
            .addresses
            .values()
            .filter_map(|addr_info| {
                let p2pkh = PlatformP2PKHAddress::from_address(&addr_info.address).ok()?;
                let balance = account.address_credit_balance(&p2pkh);
                Some((PlatformAddress::P2pkh(p2pkh.to_bytes()), balance))
            })
            .collect();
        let candidates = build_auto_select_candidates(
            address_balances.iter().copied(),
            outputs,
            min_input_amount,
        );

        // Classify empty-candidates failure into a typed diagnostic.
        if candidates.is_empty() {
            if let Some(err) = detect_no_selectable_inputs(
                address_balances.iter().copied(),
                outputs,
                min_input_amount,
            ) {
                return Err(err);
            }
        }

        let selected = match fee_strategy {
            [AddressFundsFeeStrategyStep::DeductFromInput(0)] => select_inputs_deduct_from_input(
                candidates,
                outputs,
                total_output,
                fee_strategy,
                platform_version,
            )?,
            [AddressFundsFeeStrategyStep::ReduceOutput(0)] => select_inputs_reduce_output(
                candidates,
                outputs,
                total_output,
                fee_strategy,
                platform_version,
            )?,
            _ => {
                return Err(PlatformWalletError::AddressOperation(
                    "auto_select_inputs supports fee_strategy = [DeductFromInput(0)] \
                     or [ReduceOutput(0)]; other shapes must use InputSelection::Explicit"
                        .to_string(),
                ))
            }
        };

        // Gate the FINAL selected set against the DPP per-transition input cap.
        // This is the single chokepoint reached after BOTH the
        // `[DeductFromInput(0)]` and `[ReduceOutput(0)]` selectors, where
        // `selected.len()` equals the input count `transfer` will sign — so the
        // cap is enforced for every Auto fee-strategy shape.
        enforce_max_address_inputs(&selected, platform_version)?;

        Ok(selected)
    }

    /// Simulate the fee strategy to determine how much additional balance
    /// the inputs need beyond the output amounts. Walks the strategy steps
    /// in order and returns the residual fee inputs must cover.
    fn estimate_fee_for_inputs(
        input_count: usize,
        output_count: usize,
        fee_strategy: &[AddressFundsFeeStrategyStep],
        outputs: &BTreeMap<PlatformAddress, Credits>,
        platform_version: &PlatformVersion,
    ) -> Credits {
        let total_fee = AddressFundsTransferTransition::estimate_min_fee(
            input_count,
            output_count,
            platform_version,
        );

        let mut remaining_fee = total_fee;
        let output_amounts: Vec<Credits> = outputs.values().copied().collect();

        for step in fee_strategy {
            if remaining_fee == 0 {
                break;
            }
            match step {
                AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                    if let Some(&amount) = output_amounts.get(*index as usize) {
                        let reduction = remaining_fee.min(amount);
                        remaining_fee -= reduction;
                    }
                }
                AddressFundsFeeStrategyStep::DeductFromInput(_) => {
                    break;
                }
            }
        }

        remaining_fee
    }
}

/// Build the auto-selection candidate list: keep only addresses whose balance
/// reaches `min_input_amount`, drop any address that is also a destination
/// output (the protocol forbids the same address being both input and output),
/// then sort balance-descending so the selector picks the smallest covering
/// prefix.
fn build_auto_select_candidates<I>(
    address_balances: I,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    min_input_amount: Credits,
) -> Vec<(PlatformAddress, Credits)>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let mut candidates: Vec<(PlatformAddress, Credits)> = address_balances
        .into_iter()
        .filter(|(addr, balance)| *balance >= min_input_amount && !outputs.contains_key(addr))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates
}

/// Enforce DPP's per-transition input cap on the FINAL Auto-selected input set.
///
/// DPP's `AddressFundsTransferTransition` v0 validator rejects the whole
/// transition when `inputs.len() > max_address_inputs` (16 on v2/v3) with
/// `TransitionOverMaxInputsError` — see `validate_structure` in
/// `address_funds/address_funds_transfer_transition/v0/state_transition_validation.rs`.
/// The Auto path produces exactly one input per selected address (the same map
/// `transfer` then signs), so an account whose covering prefix exceeds
/// `max_address_inputs` funded (≥ min_input) addresses would otherwise pass the
/// submit gate, sign, then deterministically fail structure validation.
///
/// We ERROR rather than auto-cap: capping would change the "cover the requested
/// output" contract (it would silently fund less than the caller asked for) and
/// is a product decision out of scope. The typed `AddressOperation` carries
/// consolidate/smaller-amount guidance, mirroring the withdrawal planner's
/// `reserve_withdrawal_fee_on_largest_input` input-count cap.
fn enforce_max_address_inputs(
    selected: &BTreeMap<PlatformAddress, Credits>,
    platform_version: &PlatformVersion,
) -> Result<(), PlatformWalletError> {
    let max_address_inputs = platform_version.dpp.state_transitions.max_address_inputs as usize;
    if selected.len() > max_address_inputs {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Too many funded addresses to cover this transfer at once: {} addresses are \
             needed but the protocol allows at most {} inputs per transfer. Consolidate \
             funds onto fewer addresses, or send a smaller amount.",
            selected.len(),
            max_address_inputs
        )));
    }
    Ok(())
}

/// Classify why no candidate survived the filter. Returns `None` when no
/// funded address exists at all (caller falls through to generic
/// insufficient-balance); otherwise returns the dominant failure shape.
/// When both apply, `OnlyOutputAddressesFunded` wins — rotating the receive
/// address is the typically more actionable fix.
fn detect_no_selectable_inputs<I>(
    address_balances: I,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    min_input_amount: Credits,
) -> Option<PlatformWalletError>
where
    I: IntoIterator<Item = (PlatformAddress, Credits)>,
{
    let mut funded_outputs: Vec<PlatformAddress> = Vec::new();
    let mut sub_min_count: usize = 0;
    let mut sub_min_aggregate: Credits = 0;
    for (addr, balance) in address_balances {
        if balance >= min_input_amount {
            if outputs.contains_key(&addr) {
                funded_outputs.push(addr);
            }
        } else if balance > 0 {
            sub_min_count = sub_min_count.saturating_add(1);
            sub_min_aggregate = sub_min_aggregate.saturating_add(balance);
        }
    }
    if !funded_outputs.is_empty() {
        return Some(PlatformWalletError::OnlyOutputAddressesFunded {
            funded_outputs,
            sub_min_count,
            sub_min_aggregate,
            min_input_amount,
        });
    }
    if sub_min_count > 0 {
        return Some(PlatformWalletError::OnlyDustInputs {
            sub_min_count,
            sub_min_aggregate,
            min_input_amount,
        });
    }
    None
}

/// `[DeductFromInput(0)]` selector. Defensively re-sorts `candidates`
/// balance-descending before growing the covering prefix, so test and
/// future direct callers can pass any order without silently picking a
/// larger-than-needed prefix.
///
/// Produces an inputs map satisfying:
/// 1. `Σ selected.values() == total_output`.
/// 2. The `DeductFromInput(0)` fee target — the lex-smallest entry, which is
///    the `BTreeMap` index-0 — must keep `balance − consumed ≥ estimated_fee`
///    so drive can deduct the fee from its remaining balance (otherwise
///    `fee_fully_covered = false` and the transition is rejected).
///
/// Algorithm:
/// 1. Grow the prefix until `Σ balances ≥ total_output + estimated_fee`.
/// 2. Within that prefix, the lex-smallest entry is the fee target.
/// 3. Solve for `fee_target_consumed` in
///    `[max(min_input_amount, total_output − other_total),
///       fee_target_balance − estimated_fee]`. If the range is empty, extend
///    the prefix and retry; error out only when candidates are exhausted.
/// 4. Insert the fee target at its minimum consumption, then distribute the
///    remainder of `total_output` across the other prefix entries. Tail
///    consumptions below `min_input_amount` get folded back into the fee
///    target rather than producing a sub-minimum input.
/// 5. Defensive invariant checks.
fn select_inputs_deduct_from_input(
    mut candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    // Defensive sort: the prefix-grow loop assumes balance-descending order so
    // we pick the smallest covering prefix. Production callers pre-sort via
    // `build_auto_select_candidates`; this keeps direct test / future callers
    // from silently picking a worse prefix.
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    if !matches!(
        fee_strategy,
        [AddressFundsFeeStrategyStep::DeductFromInput(0)]
    ) {
        return Err(PlatformWalletError::AddressOperation(
            "select_inputs_deduct_from_input only supports fee_strategy = \
             [DeductFromInput(0)]; this is an internal helper — use \
             InputSelection::Explicit with PlatformAddressWallet::transfer for \
             other fee-strategy shapes"
                .to_string(),
        ));
    }

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    // Unsatisfiable: every input must be ≥ min_input_amount.
    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    let mut last_estimated_fee: Credits = 0;
    let mut feasible: Option<(PlatformAddress, Credits, Credits, Credits)> = None;

    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = accumulated.saturating_add(balance);

        let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
            prefix.len(),
            output_count,
            fee_strategy,
            outputs,
            platform_version,
        );
        last_estimated_fee = estimated_fee;
        let required = total_output.saturating_add(estimated_fee);

        if accumulated < required {
            continue;
        }

        let (fee_target_addr, fee_target_balance) = prefix
            .iter()
            .min_by_key(|(addr, _)| *addr)
            .copied()
            .expect("prefix is non-empty: we just pushed");

        let fee_target_max = fee_target_balance.saturating_sub(estimated_fee);
        // `accumulated` is the prefix Σ; subtracting the fee target gives Σ of peers.
        let other_total: Credits = accumulated.saturating_sub(fee_target_balance);
        let fee_target_min =
            std::cmp::max(min_input_amount, total_output.saturating_sub(other_total));

        if fee_target_min <= fee_target_max {
            feasible = Some((
                fee_target_addr,
                fee_target_balance,
                fee_target_min,
                estimated_fee,
            ));
            break;
        }
    }

    let Some((fee_target_addr, fee_target_balance, fee_target_min, estimated_fee)) = feasible
    else {
        let required_total = total_output.saturating_add(last_estimated_fee);
        if accumulated < required_total {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Insufficient balance: available {} credits, required {} \
                 (outputs {} + estimated fee {}; [DeductFromInput(0)])",
                accumulated, required_total, total_output, last_estimated_fee,
            )));
        }
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot satisfy fee headroom: no covering prefix of the available inputs \
             leaves the lex-smallest entry with ≥ estimated fee {} of remaining balance \
             after consumption. Consider providing more inputs or using a different \
             fee strategy.",
            last_estimated_fee,
        )));
    };

    // Sub-minimum tail consumptions fold back into the fee target; the
    // post-Phase-4 recompute below resolves the final fee target consumption
    // against the *actual* selected input count (QA-004). The Phase-1
    // `estimated_fee` is intentionally shadowed below.
    let _ = estimated_fee;
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

    let mut remaining = total_output.saturating_sub(fee_target_min);
    for (addr, bal) in prefix.iter() {
        if *addr == fee_target_addr {
            continue;
        }
        if remaining == 0 {
            break;
        }
        let tentative = (*bal).min(remaining);
        if tentative == 0 {
            continue;
        }
        if tentative < min_input_amount {
            // Sub-min residue folds into the fee target via the
            // post-Phase-4 recompute below; do not insert here.
            remaining = remaining.saturating_sub(tentative);
            continue;
        }
        selected.insert(*addr, tentative);
        remaining = remaining.saturating_sub(tentative);
    }

    // QA-004: Phase 1 estimates the fee against `prefix.len()`, but the
    // residue-fold above can leave `selected.len() < prefix.len()`. The
    // headroom recheck below uses the actual selected count's estimated
    // fee — over-estimating would reject feasible selections. If the
    // recomputed fee_target_min still fits within the recomputed
    // fee_target_max, keep going; otherwise we genuinely lack headroom.
    let selected_input_count = selected.len() + 1; // + fee target
    let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
        selected_input_count,
        output_count,
        fee_strategy,
        outputs,
        platform_version,
    );
    let other_total: Credits = selected.values().copied().sum();
    let fee_target_consumed =
        std::cmp::max(min_input_amount, total_output.saturating_sub(other_total));
    let fee_target_max = fee_target_balance.saturating_sub(estimated_fee);
    if fee_target_consumed > fee_target_max {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Cannot satisfy fee headroom after Phase-4 fee recompute: fee-target \
             {fee_target_addr} would consume {fee_target_consumed} (balance \
             {fee_target_balance}, max {fee_target_max} after fee {estimated_fee})",
        )));
    }

    selected.insert(fee_target_addr, fee_target_consumed);

    // Defensive post-checks: a malformed Σ or misaligned fee target ships
    // a guaranteed-rejected transition. These invariants must hold in
    // release builds too, so they're real runtime checks rather than
    // debug_assert! (which compiles out in release).
    let inputs_sum: Credits = selected.values().copied().sum();
    if inputs_sum != total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: Σ inputs ({inputs_sum}) != Σ outputs ({total_output})"
        )));
    }
    if selected.keys().next().copied() != Some(fee_target_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {fee_target_addr} is not the BTreeMap \
             index-0 (lex-smallest) entry; first entry is {:?}",
            selected.keys().next().map(|a| a.to_string()),
        )));
    }
    if fee_target_balance.saturating_sub(fee_target_consumed) < estimated_fee {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: fee target {fee_target_addr} retains {} after \
             consumption, below estimated fee {estimated_fee}",
            fee_target_balance.saturating_sub(fee_target_consumed),
        )));
    }

    Ok(selected)
}

/// `[ReduceOutput(0)]` selector. Output 0 absorbs the fee at chain time, so
/// inputs only need to sum to `total_output` — no fee headroom on inputs.
///
/// Production callers feed candidates from `build_auto_select_candidates`,
/// which already drops sub-`min_input_amount` balances; the helper also
/// guards against direct test/future-caller invocations that skip the
/// pre-filter and would otherwise produce a sub-minimum prefix entry.
///
/// **Order invariant**: the trim logic assumes balance-descending order
/// so the smallest balance lands at the end of the prefix. The selector
/// defensively re-sorts the input — test and future direct callers can
/// pass candidates in any order without silently miscomputing the
/// donor-lift in Phase 3.
///
/// Algorithm:
/// 1. Grow the prefix until `Σ balances ≥ total_output`.
/// 2. Trim the last prefix entry by `surplus = Σ − total_output` so
///    `Σ inputs == Σ outputs`. Earlier entries stay at full balance.
/// 3. If the trim drops the last entry below `min_input_amount`, shift
///    consumption from a peer in **balance-descending donor order** (largest
///    peer first) to lift it back up while keeping the donor ≥
///    `min_input_amount`. Error out if no peer has the headroom.
/// 4. Estimate the fee for the chosen input count and verify
///    `output[0] ≥ estimated_fee`; otherwise the chain-time deduction would
///    leave the fee uncovered.
/// 5. Defensive invariant checks.
fn select_inputs_reduce_output(
    mut candidates: Vec<(PlatformAddress, Credits)>,
    outputs: &BTreeMap<PlatformAddress, Credits>,
    total_output: Credits,
    fee_strategy: &[AddressFundsFeeStrategyStep],
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    // Defensive sort: the Phase-2 trim and Phase-3 donor-lift both rely on
    // balance-descending order so the smallest balance lands last. Production
    // callers already pre-sort via `build_auto_select_candidates`, but direct
    // test / future callers would otherwise silently misbehave.
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    if !matches!(fee_strategy, [AddressFundsFeeStrategyStep::ReduceOutput(0)]) {
        return Err(PlatformWalletError::AddressOperation(
            "select_inputs_reduce_output only supports fee_strategy = \
             [ReduceOutput(0)]; this is an internal helper — use \
             InputSelection::Explicit with PlatformAddressWallet::transfer for \
             other fee-strategy shapes"
                .to_string(),
        ));
    }

    let output_count = outputs.len();
    let min_input_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_input_amount;

    if total_output < min_input_amount {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Transfer amount {} is below the protocol minimum input amount {}; \
             a transfer cannot be split across inputs in a way that satisfies \
             the per-input minimum",
            total_output, min_input_amount,
        )));
    }

    let mut prefix: Vec<(PlatformAddress, Credits)> = Vec::new();
    let mut accumulated: Credits = 0;
    for (address, balance) in candidates {
        prefix.push((address, balance));
        accumulated = accumulated.saturating_add(balance);
        if accumulated >= total_output {
            break;
        }
    }

    if accumulated < total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Insufficient balance: available {} credits, required {} \
             (outputs sum; [ReduceOutput(0)] absorbs the fee from output 0)",
            accumulated, total_output,
        )));
    }

    // Module-internal guard for direct test/future-caller invocations;
    // production callers pre-filter via `build_auto_select_candidates`.
    if let Some((bad_addr, bad_balance)) = prefix
        .iter()
        .find(|(_, balance)| *balance < min_input_amount)
    {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Candidate {bad_addr} has balance {bad_balance} below \
             min_input_amount {min_input_amount}; callers must pre-filter via \
             build_auto_select_candidates before invoking the selector",
        )));
    }

    // Phase 2: every prefix entry consumes its full balance except the last,
    // which absorbs the surplus.
    let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let surplus = accumulated - total_output;
    let last_index = prefix.len() - 1;
    for (i, (addr, balance)) in prefix.iter().enumerate() {
        let consumed = if i == last_index {
            balance.saturating_sub(surplus)
        } else {
            *balance
        };
        selected.insert(*addr, consumed);
    }

    // Donor must keep ≥ `min_input_amount` itself, so its balance must reach
    // `min_input_amount + shift`. Pick the largest peer first — that is the
    // peer most likely to retain enough headroom after donating. We re-sort
    // here rather than rely on caller order to keep the donor invariant
    // local to this block.
    let last_addr = prefix[last_index].0;
    let last_consumed = selected[&last_addr];
    if last_consumed < min_input_amount && prefix.len() > 1 {
        let shift = min_input_amount - last_consumed;
        let donor_threshold = min_input_amount.saturating_add(shift);
        let mut donor_candidates: Vec<&(PlatformAddress, Credits)> = prefix
            .iter()
            .filter(|(addr, _)| *addr != last_addr)
            .collect();
        donor_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let donor_addr = donor_candidates
            .into_iter()
            .find(|(_, balance)| *balance >= donor_threshold)
            .map(|(addr, _)| *addr);
        let Some(donor_addr) = donor_addr else {
            return Err(PlatformWalletError::AddressOperation(format!(
                "Cannot satisfy per-input minimum: trimming the last input to \
                 {} (below {}) and no peer has ≥ {} of headroom to redistribute",
                last_consumed, min_input_amount, donor_threshold,
            )));
        };
        let donor_consumed = selected[&donor_addr];
        selected.insert(donor_addr, donor_consumed.saturating_sub(shift));
        selected.insert(last_addr, last_consumed.saturating_add(shift));
    }

    // TODO(platform#3040): replace with chain-time fee API. Static estimate
    // can be ~2.3x below chain-time, leaving small `output[0]` at risk.
    let estimated_fee = PlatformAddressWallet::estimate_fee_for_inputs(
        selected.len(),
        output_count,
        fee_strategy,
        outputs,
        platform_version,
    );
    let output_0 = outputs.values().next().copied().unwrap_or(0);
    if output_0 < estimated_fee {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Output 0 ({} credits) cannot absorb estimated fee ({} credits) \
             under [ReduceOutput(0)]; raise output 0 or use a different fee strategy",
            output_0, estimated_fee,
        )));
    }

    // TODO(platform#3040): drop the heuristic 3x safety band once chain-time
    // fee API lands; current ~2.3x observed gap is not a proven boundary.
    const REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE: Credits = 3;
    let safe_threshold = estimated_fee.saturating_mul(REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE);
    if output_0 < safe_threshold {
        tracing::warn!(
            output_0,
            estimated_fee,
            safety_multiple = REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
            tracking_issue = "platform#3040",
            "[ReduceOutput(0)] output 0 ({} credits) is within {}x of the static estimated \
             fee ({} credits); chain-time fee may exceed the static estimate (platform#3040), \
             risking on-chain rejection. Consider raising output 0 or switching to \
             [DeductFromInput(0)].",
            output_0,
            REDUCE_OUTPUT_FEE_SAFETY_MULTIPLE,
            estimated_fee,
        );
    }

    let inputs_sum: Credits = selected.values().copied().sum();
    if inputs_sum != total_output {
        return Err(PlatformWalletError::AddressOperation(format!(
            "Internal selection error: Σ inputs ({inputs_sum}) != Σ outputs ({total_output})"
        )));
    }

    Ok(selected)
}

/// Reject `change_addr` collisions before the chain does: the protocol
/// errors deterministically when a transition has the same address as
/// both input and output, and silently merging into a caller-declared
/// output would mask a destination amount. Called at every
/// `transfer_with_change_address` entry that has the inputs map in scope.
fn validate_change_address<'a, I>(
    change_addr: &PlatformAddress,
    user_outputs: &BTreeMap<PlatformAddress, Credits>,
    inputs: I,
) -> Result<(), PlatformWalletError>
where
    I: IntoIterator<Item = &'a PlatformAddress>,
{
    if user_outputs.contains_key(change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} already appears in user_outputs; \
             refusing to silently merge a change-output amount into a caller-declared \
             output. Pick a fresh change_addr.",
        )));
    }
    if inputs.into_iter().any(|addr| addr == change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} also appears in the input map; \
             the protocol rejects transitions where the same address is both input \
             and output. Pick a fresh change_addr.",
        )));
    }
    Ok(())
}

/// Augment `user_outputs` with an explicit change output absorbing the
/// surplus `Σ inputs − Σ user_outputs`. Caller MUST invoke
/// [`validate_change_address`] first to rule out collisions; this fn
/// re-checks the user_outputs side defensively, rejects the no-surplus
/// case, and rejects residuals below the protocol per-output minimum
/// (`OutputBelowMinimumError`, code 10810).
fn augment_outputs_with_change(
    mut user_outputs: BTreeMap<PlatformAddress, Credits>,
    change_addr: PlatformAddress,
    input_sum: Credits,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    if user_outputs.contains_key(&change_addr) {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address {change_addr:?} already appears in user_outputs; \
             refusing to silently merge a change-output amount into a caller-declared \
             output. Pick a fresh change_addr.",
        )));
    }
    let user_output_sum: Credits = saturating_sum_credits(user_outputs.values().copied());
    if input_sum <= user_output_sum {
        return Err(PlatformWalletError::AddressOperation(format!(
            "output_change_address: Some(_) requires Σ inputs ({input_sum}) > \
             Σ user_outputs ({user_output_sum}); no surplus to route as change. \
             Drop output_change_address or grow the input map.",
        )));
    }
    let change_amount = input_sum.saturating_sub(user_output_sum);
    let min_output_amount = platform_version
        .dpp
        .state_transitions
        .address_funds
        .min_output_amount;
    if change_amount < min_output_amount {
        return Err(PlatformWalletError::ChangeBelowMinimumOutput {
            change_amount,
            min_output_amount,
        });
    }
    user_outputs.insert(change_addr, change_amount);
    Ok(user_outputs)
}

#[cfg(test)]
mod auto_select_tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::StateTransitionStructureValidation;
    use dpp::version::LATEST_PLATFORM_VERSION;
    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    fn outputs_for(target: PlatformAddress, amount: Credits) -> BTreeMap<PlatformAddress, Credits> {
        std::iter::once((target, amount)).collect()
    }

    /// Feed a selector result into dpp's `validate_structure` to confirm the
    /// transition is shape-valid. Uses zero nonces and dummy P2PKH witnesses.
    fn assert_selection_validates(
        selected: &BTreeMap<PlatformAddress, Credits>,
        outputs: &BTreeMap<PlatformAddress, Credits>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        platform_version: &PlatformVersion,
    ) {
        let inputs = selected
            .iter()
            .map(|(addr, amount)| (*addr, (0u32, *amount)))
            .collect();
        let input_witnesses = (0..selected.len())
            .map(|_| AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            })
            .collect();
        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs: outputs.clone(),
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses,
        };
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "validate_structure rejected the selection: {:?}",
            result.errors,
        );
    }

    /// One address with a large balance, output amount well below it →
    /// `selected[addr] == total_output` (NOT full balance, NOT `total_output + fee`).
    /// Fee comes from the address's remaining balance via `DeductFromInput(0)`.
    #[test]
    fn single_input_oversized_balance_trims_to_output_amount() {
        let addr = p2pkh(0x11);
        let target = p2pkh(0x22);
        let outputs = outputs_for(target, 10_000_000);
        let total_output = 10_000_000u64;
        let candidates = vec![(addr, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(selected.get(&addr), Some(&10_000_000));
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, outputs.values().copied().sum::<Credits>());

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Balance-descending input — the order `auto_select_inputs` supplies —
    /// with a single largest balance covering `total_output + fee` produces a
    /// 1-input map.
    #[test]
    fn descending_order_picks_single_largest_when_sufficient() {
        let addr_small = p2pkh(0x01);
        let addr_large = p2pkh(0xFE);
        let target = p2pkh(0xCC);
        let total_output = 30_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_large, 100_000_000), (addr_small, 5_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[&addr_large], total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Protocol-level proof: the inputs map a naive selector would produce
    /// for `(20M, 50M)` / `total_output = 30M` / `[DeductFromInput(0)]`
    /// (`{addr_a: 20M, addr_b: 10M}`), when fed to
    /// `deduct_fee_from_outputs_or_remaining_balance_of_inputs`, returns
    /// `fee_fully_covered = false` — drive's `validate_fees_of_event` would
    /// reject the transition. The fixed selector retains `min_input_amount`
    /// at addr_a so the fee deduction has headroom.
    #[test]
    fn pre_fix_buggy_selector_output_is_rejected_by_protocol_fee_deduction() {
        use dpp::address_funds::fee_strategy::deduct_fee_from_inputs_and_outputs::deduct_fee_from_outputs_or_remaining_balance_of_inputs;
        use dpp::prelude::AddressNonce;

        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0xFF);
        let total_output = 30_000_000u64;
        let addr_a_balance = 20_000_000u64;
        let addr_b_balance = 50_000_000u64;
        let outputs = outputs_for(target, total_output);
        let fee_strategy: AddressFundsFeeStrategy =
            vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let mut buggy_inputs_consumed: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        buggy_inputs_consumed.insert(addr_a, 20_000_000);
        buggy_inputs_consumed.insert(addr_b, 10_000_000);

        let mut input_current_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            BTreeMap::new();
        input_current_balances.insert(addr_a, (0, addr_a_balance - 20_000_000));
        input_current_balances.insert(addr_b, (0, addr_b_balance - 10_000_000));

        let fee: Credits = 1_000_000;
        let added_to_outputs: BTreeMap<PlatformAddress, Credits> = outputs.clone();

        let result = deduct_fee_from_outputs_or_remaining_balance_of_inputs(
            input_current_balances.clone(),
            added_to_outputs,
            &fee_strategy,
            fee,
            pv,
        )
        .expect("deduction call must succeed (rejection is via fee_fully_covered)");

        assert!(
            !result.fee_fully_covered,
            "Pre-fix selector's output must be rejected by the protocol's fee deduction"
        );
        assert!(addr_b_balance - 10_000_000 >= fee);

        // Cross-check: the fixed selector at the same fixture produces a
        // map drive's fee deduction accepts. The defensive
        // balance-descending sort (QA-003) puts addr_b first; addr_b alone
        // covers `total_output + fee`, so the single-input prefix
        // `{addr_b: 30_000_000}` is what the fixed selector ships, and
        // addr_b retains 20M of headroom for the fee.
        let fixed = select_inputs_deduct_from_input(
            vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)],
            &outputs,
            total_output,
            &fee_strategy,
            pv,
        )
        .expect("fixed selector");
        assert_eq!(fixed.get(&addr_b), Some(&total_output));
        assert!(!fixed.contains_key(&addr_a));
        assert_selection_validates(&fixed, &outputs, fee_strategy, pv);
    }

    /// Phase 1 covers `total_output + fee` but the lex-smallest entry has no
    /// headroom for the fee. Selection must error out rather than ship a
    /// transition the validator will reject.
    ///
    /// Fixture: the lex-smallest address has a balance below `min_input + fee`
    /// (so its `fee_target_min` saturates to `min_input` while
    /// `fee_target_max = balance − fee` is below that), and the larger peer
    /// undershoots covering alone — together they cover, but the
    /// lex-smallest entry cannot retain `estimated_fee` after consumption.
    #[test]
    fn fee_headroom_violation_errors() {
        let addr_tiny = p2pkh(0x01); // lex-smallest → fee target after the prefix grows
        let addr_b = p2pkh(0xA0);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;

        let total_output = 30_000_000u64;
        // addr_b alone undershoots `total_output + fee_1in ≈ 36.5M`, so the
        // prefix must include addr_tiny.
        let addr_b_balance = 35_000_000u64;
        // addr_tiny < fee_1in + min_input ≈ 6.6M → no fee headroom after the
        // sub-min-floor consumption.
        let addr_tiny_balance = 6_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_tiny, addr_tiny_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("expected fee-headroom error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("Cannot satisfy fee headroom"), "got {msg:?}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Tail entry's tentative consumption falls below `min_input_amount`. The
    /// selector folds the residue back into the fee target so every shipped
    /// input ≥ `min_input_amount`.
    #[test]
    fn non_fee_target_below_min_input_redistributes() {
        let addr_x = p2pkh(0x01); // lex-smallest → fee target
        let addr_y = p2pkh(0x02); // sub-min peer; folds into fee target
        let addr_z = p2pkh(0x03); // large peer; absorbs the bulk
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Fixture (numbers chosen against fee schedule `500_000*N + 6_000_000`):
        // - prefix [x] (acc 10M) doesn't cover 10.5M (=4M+fee_1in).
        // - prefix [x,y] (acc 10.08M) doesn't cover 11M (=4M+fee_2in).
        // - prefix [x,y,z] (acc 12.08M) covers 11.5M.
        // - Phase 4: y's tentative=80k folds into fee target; z absorbs 2M.
        let total_output = 4_000_000u64;
        let addr_x_balance = 10_000_000u64;
        let addr_y_balance = 80_000u64; // below min_input_amount (100_000)
        let addr_z_balance = 2_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![
            (addr_x, addr_x_balance),
            (addr_y, addr_y_balance),
            (addr_z, addr_z_balance),
        ];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("redistribute path must reach Ok");

        for (addr, amount) in selected.iter() {
            assert!(*amount >= min_input, "{addr} consumes {amount}");
        }
        assert!(
            !selected.contains_key(&addr_y),
            "sub-min y must be folded out"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// QA-004: Phase 1 sizes `estimated_fee` against `prefix.len()`, but the
    /// residue-fold can leave `selected.len() < prefix.len()`. The post-Phase-4
    /// recompute against the *actual* selected count must use the smaller fee
    /// — otherwise feasible selections get falsely rejected as
    /// "Cannot satisfy fee headroom".
    ///
    /// Fixture: 3-entry prefix; the middle sub-min entry folds into the fee
    /// target, so `selected = {fee_target, addr_z}` has len 2. The Phase-1
    /// fee_3in would over-bound the fee target's headroom; the post-Phase-4
    /// recompute with fee_2in must succeed.
    #[test]
    fn fee_recompute_after_residue_fold_succeeds() {
        let addr_x = p2pkh(0x01); // lex-smallest → fee target
        let addr_y = p2pkh(0x02); // sub-min residue, folds away
        let addr_z = p2pkh(0x03); // peer
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Numbers: same shape as `non_fee_target_below_min_input_redistributes`
        // — prefix [x,y,z] is needed by Phase-1 fee_3in, but final selected
        // is {x,z} so fee_2in applies. Both paths converge here because the
        // headroom is large; this asserts no false rejection.
        let total_output = 4_000_000u64;
        let candidates = vec![
            (addr_x, 10_000_000u64),
            (addr_y, 80_000u64), // < min_input → folds into fee target
            (addr_z, 2_000_000u64),
        ];
        let outputs = outputs_for(target, total_output);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let selected =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("post-Phase-4 fee recompute must accept the selection");

        assert_eq!(selected.len(), 2, "y folded into fee target");
        assert!(!selected.contains_key(&addr_y));
        for (addr, amount) in &selected {
            assert!(*amount >= min_input, "{addr} consumes {amount}");
        }
        assert_eq!(selected.values().copied().sum::<Credits>(), total_output);
        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// QA-001: an address that is also a destination output must be excluded
    /// from auto-selection candidates, even when it is the only address with
    /// sufficient balance. Otherwise the protocol would reject the transition
    /// with `Output address cannot also be an input address`.
    #[test]
    fn auto_select_inputs_excludes_output_addresses() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_a = p2pkh(0xA1);
        let addr_b = p2pkh(0xB2);
        let outputs = outputs_for(addr_a, min_input);

        let address_balances = vec![(addr_a, min_input * 3), (addr_b, min_input / 2)];
        let candidates =
            build_auto_select_candidates(address_balances.clone(), &outputs, min_input);
        assert!(candidates.is_empty(), "got {candidates:?}");

        let no_outputs = BTreeMap::new();
        let with_self_spend =
            build_auto_select_candidates(address_balances, &no_outputs, min_input);
        assert_eq!(with_self_spend, vec![(addr_a, min_input * 3)]);
    }

    /// Empty candidate list → error rather than panic / silent zero-input transition.
    #[test]
    fn no_candidates_errors() {
        let target = p2pkh(0x55);
        let outputs = outputs_for(target, 1_000_000);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let err =
            select_inputs_deduct_from_input(Vec::new(), &outputs, 1_000_000, &fee_strategy, pv)
                .expect_err("expected error for empty candidates");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    /// A covering input set larger than `max_address_inputs` must NOT be
    /// shippable: DPP's v0 validator rejects the whole transition with
    /// `TransitionOverMaxInputsError` once `inputs.len()` exceeds the cap. The
    /// Auto path uses one input per selected address, so the
    /// `enforce_max_address_inputs` gate (called in `auto_select_inputs` after
    /// the selector returns) must surface this as a typed "too many inputs"
    /// error rather than letting `transfer` sign a guaranteed-rejected
    /// transition. Mirrors withdrawal's `plan_more_than_max_inputs_cant_fund`.
    /// We build the final selected map directly (one input per address, one
    /// more than the cap) since the gate operates on the post-selection set.
    #[test]
    fn auto_select_more_than_max_inputs_cant_fund() {
        let pv = LATEST_PLATFORM_VERSION;
        let max_inputs = pv.dpp.state_transitions.max_address_inputs as usize;

        // One input per address, one more than the cap. The amounts are
        // irrelevant to the count cap — only `selected.len()` matters.
        let mut selected: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        for i in 0..=max_inputs {
            selected.insert(p2pkh(i as u8), 1_000_000u64);
        }
        assert_eq!(
            selected.len(),
            max_inputs + 1,
            "test setup: must hold one more input than the cap"
        );

        let err = enforce_max_address_inputs(&selected, pv)
            .expect_err("more than max_address_inputs selected inputs must not be fundable");
        match err {
            PlatformWalletError::AddressOperation(msg) => assert!(
                msg.contains("at most") && msg.contains("inputs"),
                "expected a too-many-inputs message, got: {msg}"
            ),
            other => panic!("expected AddressOperation too-many-inputs, got {other:?}"),
        }

        // Exactly at the cap must pass the gate (boundary check).
        let mut at_cap: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        for i in 0..max_inputs {
            at_cap.insert(p2pkh(i as u8), 1_000_000u64);
        }
        assert_eq!(at_cap.len(), max_inputs);
        enforce_max_address_inputs(&at_cap, pv)
            .expect("an input set exactly at the cap must be allowed");
    }

    /// `total_output < min_input_amount` is unsatisfiable. The selector must
    /// reject upfront with a descriptive error.
    #[test]
    fn total_output_below_min_input_amount_errors() {
        let addr = p2pkh(0x10);
        let target = p2pkh(0x90);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;
        let total_output = min_input - 1;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 100_000_000)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

        let err =
            select_inputs_deduct_from_input(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("expected below-min-input error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("below the protocol minimum input amount"),
                    "{msg:?}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Single input fully covers `total_output`; the input is trimmed to
    /// `total_output` (no fee headroom on inputs — output 0 absorbs the fee
    /// at chain time).
    #[test]
    fn reduce_output_happy_path_single_input() {
        let addr = p2pkh(0x11);
        let target = p2pkh(0x22);
        let total_output = 10_000_000u64;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr, 100_000_000u64)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];
        let pv = LATEST_PLATFORM_VERSION;

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("selection");

        assert_eq!(selected.get(&addr), Some(&total_output));
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output);

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// Detector returns `OnlyOutputAddressesFunded` when every funded address
    /// is also a destination, and `OnlyDustInputs` when every funded balance
    /// is below `min_input_amount`. When both shapes apply simultaneously,
    /// the address-collision signal wins (more actionable fix).
    #[test]
    fn detect_no_selectable_inputs_classifies_failure_shape() {
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let addr_out = p2pkh(0xC3);
        let addr_dust = p2pkh(0xD4);
        let outputs = outputs_for(addr_out, min_input);

        // Output-collision only.
        let collision_only = [(addr_out, min_input * 5)];
        match detect_no_selectable_inputs(collision_only.iter().copied(), &outputs, min_input)
            .expect("collision case")
        {
            PlatformWalletError::OnlyOutputAddressesFunded {
                funded_outputs,
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            } => {
                assert_eq!(funded_outputs, vec![addr_out]);
                assert_eq!(sub_min_count, 0);
                assert_eq!(sub_min_aggregate, 0);
                assert_eq!(min_input_amount, min_input);
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }

        // Dust only.
        let no_outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        let dust_only = [(addr_dust, min_input / 3)];
        match detect_no_selectable_inputs(dust_only.iter().copied(), &no_outputs, min_input)
            .expect("dust case")
        {
            PlatformWalletError::OnlyDustInputs {
                sub_min_count,
                sub_min_aggregate,
                min_input_amount,
            } => {
                assert_eq!(sub_min_count, 1);
                assert_eq!(sub_min_aggregate, min_input / 3);
                assert_eq!(min_input_amount, min_input);
            }
            other => panic!("expected OnlyDustInputs, got {other:?}"),
        }

        // Both: collision wins, dust info is preserved as auxiliary fields.
        let both = [(addr_out, min_input * 5), (addr_dust, min_input / 3)];
        match detect_no_selectable_inputs(both.iter().copied(), &outputs, min_input)
            .expect("combined case")
        {
            PlatformWalletError::OnlyOutputAddressesFunded {
                funded_outputs,
                sub_min_count,
                sub_min_aggregate,
                ..
            } => {
                assert_eq!(funded_outputs, vec![addr_out]);
                assert_eq!(sub_min_count, 1);
                assert_eq!(sub_min_aggregate, min_input / 3);
            }
            other => panic!("expected OnlyOutputAddressesFunded, got {other:?}"),
        }

        // No funded address → None (caller falls through to insufficient-balance).
        let no_funds = [(addr_out, 0u64), (addr_dust, 0u64)];
        assert!(
            detect_no_selectable_inputs(no_funds.iter().copied(), &outputs, min_input).is_none()
        );
    }

    /// PA-001b: the change-address override must add exactly one extra output
    /// absorbing `Σ inputs − Σ user_outputs`, leaving `Σ inputs == Σ outputs`
    /// so the protocol's structural invariant still holds.
    #[test]
    fn augment_outputs_with_change_adds_residual_output() {
        let user_target = p2pkh(0x22);
        let change_addr = p2pkh(0x33);
        let user_outputs = outputs_for(user_target, 5_000_000);
        let pv = LATEST_PLATFORM_VERSION;
        let outputs = augment_outputs_with_change(user_outputs, change_addr, 60_000_000, pv)
            .expect("augment");
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs.get(&user_target), Some(&5_000_000));
        assert_eq!(
            outputs.get(&change_addr),
            Some(&55_000_000),
            "change output must absorb exactly the surplus"
        );
        let output_sum: Credits = outputs.values().sum();
        assert_eq!(
            output_sum, 60_000_000,
            "Σ outputs must equal input sum (Σ inputs == Σ outputs invariant)"
        );
    }

    /// PA-001b: the override must reject a `change_addr` that already appears
    /// in the caller's user outputs to prevent a silent merge.
    #[test]
    fn augment_outputs_with_change_rejects_duplicate_address() {
        let target = p2pkh(0x44);
        let user_outputs = outputs_for(target, 5_000_000);
        let pv = LATEST_PLATFORM_VERSION;
        let err = augment_outputs_with_change(user_outputs, target, 60_000_000, pv)
            .expect_err("change_addr equal to user output must be rejected");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("already appears in user_outputs"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// PA-001b: when `Σ user_outputs ≥ Σ inputs` there is no surplus to route.
    /// The wrapper must reject rather than emit a zero-credit (or underflowing)
    /// change output.
    #[test]
    fn augment_outputs_with_change_rejects_no_surplus() {
        let target = p2pkh(0x55);
        let change_addr = p2pkh(0x66);
        let user_outputs = outputs_for(target, 60_000_000);
        let pv = LATEST_PLATFORM_VERSION;
        let err = augment_outputs_with_change(user_outputs, change_addr, 60_000_000, pv)
            .expect_err("equal sums must be rejected: nothing to route as change");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("no surplus"), "unexpected message: {msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// QA-001: residual in the `(0, min_output_amount)` band must be rejected
    /// before the chain does (`OutputBelowMinimumError`, code 10810). The
    /// existing tests cover residual=0 (no-surplus) and residual=55M (well
    /// above min); this fills the gap in the middle.
    #[test]
    fn augment_outputs_with_change_rejects_sub_minimum_residual() {
        let target = p2pkh(0x77);
        let change_addr = p2pkh(0x88);
        let pv = LATEST_PLATFORM_VERSION;
        let min_output = pv.dpp.state_transitions.address_funds.min_output_amount;
        // Pick a residual strictly between 0 and min_output_amount.
        let residual = min_output - 1;
        let user_output_amount = 5_000_000u64;
        let user_outputs = outputs_for(target, user_output_amount);
        let input_sum = user_output_amount + residual;

        let err = augment_outputs_with_change(user_outputs, change_addr, input_sum, pv)
            .expect_err("sub-min residual must be rejected pre-broadcast");
        match err {
            PlatformWalletError::ChangeBelowMinimumOutput {
                change_amount,
                min_output_amount,
            } => {
                assert_eq!(change_amount, residual);
                assert_eq!(min_output_amount, min_output);
            }
            other => panic!("expected ChangeBelowMinimumOutput, got {other:?}"),
        }
    }

    /// `validate_change_address` rejects collisions with user_outputs OR
    /// the input map; otherwise it accepts.
    #[test]
    fn validate_change_address_rejects_both_collision_shapes() {
        let target = p2pkh(0xAA);
        let input = p2pkh(0xBB);
        let other = p2pkh(0xCC);
        let user_outputs = outputs_for(target, 5_000_000);
        let input_keys = std::iter::once(&input);

        // Collision with user_outputs.
        let err = validate_change_address(&target, &user_outputs, input_keys.clone())
            .expect_err("user_outputs collision");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("already appears in user_outputs"), "{msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }

        // Collision with inputs.
        let err = validate_change_address(&input, &user_outputs, input_keys.clone())
            .expect_err("input collision");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(msg.contains("also appears in the input map"), "{msg}");
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }

        // Fresh address — accepted.
        validate_change_address(&other, &user_outputs, input_keys)
            .expect("fresh change_addr must validate");
    }

    /// `[ReduceOutput(0)]` Phase 3 success: two-input prefix where the trim
    /// drops the last entry below `min_input_amount` and the donor has the
    /// headroom to lift it back. Verifies the shift lands both entries
    /// exactly at min_input and the donor's remaining consumption stays
    /// ≥ min_input.
    #[test]
    fn reduce_output_phase3_donor_lift_success() {
        let addr_a = p2pkh(0xA1); // larger balance → donor
        let addr_b = p2pkh(0xB2); // smaller balance → last (gets trimmed)
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // candidates are caller-supplied in balance-descending order.
        // total_output leaves only 10_000 on the last entry after trim,
        // forcing Phase 3 donor lift up to `min_input`.
        let addr_a_balance = 109_000_000u64;
        let addr_b_balance = min_input; // exactly min_input
        let total_output = addr_a_balance + min_input - 90_000;
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let selected =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect("Phase 3 donor lift must succeed");

        // Both entries end exactly at expected post-shift values.
        let consumed_a = selected[&addr_a];
        let consumed_b = selected[&addr_b];
        assert_eq!(consumed_b, min_input, "last lifted back to min_input");
        assert_eq!(
            consumed_a,
            addr_a_balance - 90_000,
            "donor consumes its balance minus the shift"
        );
        let input_sum: Credits = selected.values().sum();
        assert_eq!(input_sum, total_output, "Σ inputs == total_output");

        assert_selection_validates(&selected, &outputs, fee_strategy, pv);
    }

    /// `[ReduceOutput(0)]` Phase 3 failure: trim drops the last entry below
    /// `min_input_amount` and no peer has the headroom to donate. The
    /// selector must error out rather than ship a sub-minimum input.
    #[test]
    fn reduce_output_phase3_redistribution_fails_when_no_donor() {
        let addr_a = p2pkh(0x01); // donor candidate (only peer); exactly min_input → no headroom
        let addr_b = p2pkh(0x02); // last entry, gets trimmed below min_input
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        // Both candidates have exactly min_input balance. Prefix covers
        // total_output by including both; the trim drops addr_b below
        // min_input and addr_a (min_input == 100_000) cannot give any
        // positive amount without dropping below min_input itself.
        let addr_a_balance = min_input;
        let addr_b_balance = min_input;
        let total_output = min_input + (min_input / 10); // 110_000: requires both inputs
        let outputs = outputs_for(target, total_output);
        let candidates = vec![(addr_a, addr_a_balance), (addr_b, addr_b_balance)];
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let err =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("no donor with headroom — must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Cannot satisfy per-input minimum"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// `[ReduceOutput(0)]` Phase 1 failure: aggregate candidate balance is
    /// below `total_output`. The selector must reject with the
    /// insufficient-balance diagnostic.
    #[test]
    fn reduce_output_phase1_insufficient_aggregate_balance() {
        let addr_a = p2pkh(0x01);
        let addr_b = p2pkh(0x02);
        let target = p2pkh(0x99);
        let pv = LATEST_PLATFORM_VERSION;
        let min_input = pv.dpp.state_transitions.address_funds.min_input_amount;

        let candidates = vec![(addr_a, min_input), (addr_b, min_input)];
        let total_output = min_input * 3; // > Σ balances (= 2 * min_input)
        let outputs = outputs_for(target, total_output);
        let fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let err =
            select_inputs_reduce_output(candidates, &outputs, total_output, &fee_strategy, pv)
                .expect_err("aggregate balance below total_output — must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("Insufficient balance"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Build a `PlatformAddressWallet` wired with a stub asset-lock manager
    /// for tests that short-circuit before any I/O. The asset-lock manager
    /// is constructed but never exercised — the rejection arms under test
    /// fire before any of its methods are called.
    fn build_short_circuit_wallet() -> crate::wallet::platform_addresses::PlatformAddressWallet {
        use crate::broadcaster::SpvBroadcaster;
        use crate::events::PlatformEventManager;
        use crate::spv::SpvRuntime;
        use crate::wallet::asset_lock::manager::AssetLockManager;
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
        use crate::wallet::platform_addresses::PlatformAddressWallet;
        use std::sync::Arc;
        use tokio::sync::{Notify, RwLock};

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let wallet_manager = Arc::new(RwLock::new(key_wallet_manager::WalletManager::new(
            sdk.network,
        )));
        let persister = WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence));
        let event_manager = Arc::new(PlatformEventManager::new(Vec::new()));
        let spv = Arc::new(SpvRuntime::new(Arc::clone(&wallet_manager), event_manager));
        let broadcaster = Arc::new(SpvBroadcaster::new(spv));
        let asset_locks = Arc::new(AssetLockManager::new(
            Arc::clone(&sdk),
            Arc::clone(&wallet_manager),
            [0u8; 32],
            Arc::new(Notify::new()),
            broadcaster,
            persister.clone(),
        ));
        PlatformAddressWallet::new(sdk, wallet_manager, [0u8; 32], asset_locks, persister)
    }

    /// CMT-007: `transfer_with_change_address` must reject
    /// `InputSelection::Auto` before doing any I/O. The rejection happens
    /// in the wrapper's own match arm, well before any wallet-manager or
    /// broadcaster method is touched.
    #[tokio::test]
    async fn transfer_with_change_address_rejects_auto_selection() {
        let wallet = build_short_circuit_wallet();
        let signer = NullSigner;
        let target = p2pkh(0x77);
        let outputs: BTreeMap<PlatformAddress, Credits> = outputs_for(target, 10_000_000);
        let change_addr = p2pkh(0x88);
        let fee_strategy = FeeStrategyByAddress::reduce_output(target);

        let err = wallet
            .transfer_with_change_address(
                0,
                InputSelection::Auto,
                outputs,
                Some(change_addr),
                fee_strategy,
                None,
                &signer,
            )
            .await
            .expect_err("Auto + Some(change_addr) must error");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("InputSelection::Explicit"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }

    /// Smoke test for the standard `transfer_with_change_address` entry: a
    /// `BTreeMap` of user outputs reaches the "Auto + Some(change_addr)"
    /// rejection arm without any I/O. Pins the public-API parameter type.
    #[tokio::test]
    async fn transfer_with_change_address_accepts_btreemap_outputs() {
        let wallet = build_short_circuit_wallet();
        let signer = NullSigner;
        let target = p2pkh(0x77);
        let outputs: BTreeMap<PlatformAddress, Credits> = outputs_for(target, 10_000_000);
        let change_addr = p2pkh(0x88);
        let fee_strategy = FeeStrategyByAddress::reduce_output(target);

        let err = wallet
            .transfer_with_change_address(
                0,
                InputSelection::Auto,
                outputs,
                Some(change_addr),
                fee_strategy,
                None,
                &signer,
            )
            .await
            .expect_err("Auto + Some(change_addr) must error");
        assert!(matches!(err, PlatformWalletError::AddressOperation(_)));
    }

    // The `inputs ∪ outputs` translation (external-recipient filtering,
    // missing-info-as-zero) now lives in the shared reconciliation seam —
    // see `build_entries_drops_unresolved_and_zeroes_missing_info` in
    // `provider.rs`.

    /// Signer used only by tests that exercise paths which short-circuit
    /// before any signing happens. Never produces a signature.
    #[derive(Debug)]
    pub(super) struct NullSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<PlatformAddress> for NullSigner {
        async fn sign(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("NullSigner used by a test path that should short-circuit before signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("NullSigner used by a test path that should short-circuit before signing")
        }

        fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
            false
        }
    }

    // -- FeeStrategyByAddress::to_indexed -----------------------------------

    /// Build a 3-row outputs map keyed by `addrs[0..3]` (assumed lex-ordered
    /// by the caller). Values are arbitrary, only the keys matter for index
    /// resolution.
    fn three_output_map(addrs: [PlatformAddress; 3]) -> BTreeMap<PlatformAddress, Credits> {
        let mut m = BTreeMap::new();
        m.insert(addrs[0], 100);
        m.insert(addrs[1], 200);
        m.insert(addrs[2], 300);
        m
    }

    fn three_input_map(addrs: [PlatformAddress; 3]) -> BTreeMap<PlatformAddress, Credits> {
        let mut m = BTreeMap::new();
        m.insert(addrs[0], 10);
        m.insert(addrs[1], 20);
        m.insert(addrs[2], 30);
        m
    }

    #[test]
    fn reduce_output_resolves_smallest_to_index_zero() {
        let a = p2pkh(0x01);
        let b = p2pkh(0x02);
        let c = p2pkh(0x03);
        let outputs = three_output_map([a, b, c]);
        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let indexed = FeeStrategyByAddress::reduce_output(a)
            .to_indexed(&inputs, &outputs)
            .expect("resolve");
        assert_eq!(indexed, vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]);
    }

    #[test]
    fn reduce_output_resolves_middle_to_correct_index() {
        let a = p2pkh(0x01);
        let b = p2pkh(0x02);
        let c = p2pkh(0x03);
        let outputs = three_output_map([a, b, c]);
        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let indexed = FeeStrategyByAddress::reduce_output(b)
            .to_indexed(&inputs, &outputs)
            .expect("resolve");
        assert_eq!(indexed, vec![AddressFundsFeeStrategyStep::ReduceOutput(1)]);
    }

    #[test]
    fn reduce_output_resolves_largest_to_last_index() {
        let a = p2pkh(0x01);
        let b = p2pkh(0x02);
        let c = p2pkh(0x03);
        let outputs = three_output_map([a, b, c]);
        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let indexed = FeeStrategyByAddress::reduce_output(c)
            .to_indexed(&inputs, &outputs)
            .expect("resolve");
        assert_eq!(indexed, vec![AddressFundsFeeStrategyStep::ReduceOutput(2)]);
    }

    #[test]
    fn deduct_from_input_resolves_at_each_position() {
        let a = p2pkh(0x10);
        let b = p2pkh(0x20);
        let c = p2pkh(0x30);
        let inputs = three_input_map([a, b, c]);
        let outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let cases = [(a, 0u16), (b, 1u16), (c, 2u16)];
        for (addr, expected) in cases {
            let indexed = FeeStrategyByAddress::deduct_from_input(addr)
                .to_indexed(&inputs, &outputs)
                .expect("resolve");
            assert_eq!(
                indexed,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(expected)]
            );
        }
    }

    #[test]
    fn multi_step_resolves_each_independently() {
        let in_a = p2pkh(0x10);
        let in_b = p2pkh(0x20);
        let out_a = p2pkh(0x40);
        let out_b = p2pkh(0x50);
        let inputs = {
            let mut m = BTreeMap::new();
            m.insert(in_a, 10u64);
            m.insert(in_b, 20u64);
            m
        };
        let outputs = {
            let mut m = BTreeMap::new();
            m.insert(out_a, 100u64);
            m.insert(out_b, 200u64);
            m
        };

        let strategy = FeeStrategyByAddress(vec![
            FeeStrategyStepByAddress::DeductFromInputAddress(in_b),
            FeeStrategyStepByAddress::ReduceOutputAddress(out_b),
        ]);
        let indexed = strategy.to_indexed(&inputs, &outputs).expect("resolve");
        assert_eq!(
            indexed,
            vec![
                AddressFundsFeeStrategyStep::DeductFromInput(1),
                AddressFundsFeeStrategyStep::ReduceOutput(1),
            ]
        );
    }

    #[test]
    fn unknown_input_address_yields_input_not_found() {
        let known = p2pkh(0x10);
        let unknown = p2pkh(0xFF);
        let mut inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        inputs.insert(known, 10);
        let outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let err = FeeStrategyByAddress::deduct_from_input(unknown)
            .to_indexed(&inputs, &outputs)
            .expect_err("must fail");
        assert!(matches!(
            err,
            FeeStrategyResolveError::InputAddressNotFound(addr) if addr == unknown
        ));
    }

    #[test]
    fn unknown_output_address_yields_output_not_found() {
        let known = p2pkh(0x40);
        let unknown = p2pkh(0xFF);
        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        let mut outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        outputs.insert(known, 100);

        let err = FeeStrategyByAddress::reduce_output(unknown)
            .to_indexed(&inputs, &outputs)
            .expect_err("must fail");
        assert!(matches!(
            err,
            FeeStrategyResolveError::OutputAddressNotFound(addr) if addr == unknown
        ));
    }

    /// Regression: with `user_outputs = { 0x02, 0x03 }` and a lex-smaller
    /// change address `0x01` inserted by augmentation, a fee strategy
    /// targeting `0x03` must resolve to index **2**, not 1. The pre-refactor
    /// indexed API silently produced index 1 (caller's pre-augmentation
    /// position), misrouting the fee. The address-keyed API resolves against
    /// the final outputs map and produces index 2.
    #[test]
    fn reduce_output_handles_non_zero_index_shift_after_augmentation() {
        let change = p2pkh(0x01);
        let user_a = p2pkh(0x02);
        let user_b = p2pkh(0x03);

        // Final outputs as seen by the signer (post-augmentation).
        let mut final_outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        final_outputs.insert(change, 50);
        final_outputs.insert(user_a, 100);
        final_outputs.insert(user_b, 200);

        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let indexed = FeeStrategyByAddress::reduce_output(user_b)
            .to_indexed(&inputs, &final_outputs)
            .expect("resolve");
        assert_eq!(
            indexed,
            vec![AddressFundsFeeStrategyStep::ReduceOutput(2)],
            "user_b is at index 2 of the post-augmentation lex order, not 1"
        );
    }

    /// The old wrapper rejected `[ReduceOutput(0)]` when the change address
    /// was lex-smaller than every user output, because it assumed the index
    /// referred to the caller's mental model rather than the final map. With
    /// the address-keyed API this is now a legitimate, supported call:
    /// targeting the change row by address resolves to whatever index it
    /// occupies in the final outputs map.
    #[test]
    fn reduce_output_targeting_lex_smallest_change_address_is_allowed() {
        let change = p2pkh(0x01);
        let user_a = p2pkh(0x02);
        let user_b = p2pkh(0x03);

        let mut final_outputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        final_outputs.insert(change, 50);
        final_outputs.insert(user_a, 100);
        final_outputs.insert(user_b, 200);

        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();

        let indexed = FeeStrategyByAddress::reduce_output(change)
            .to_indexed(&inputs, &final_outputs)
            .expect("resolve must succeed; old heuristic is gone");
        assert_eq!(indexed, vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]);
    }

    /// CMT-003: when the wrapper resolves `ReduceOutputAddress` to an output
    /// row whose amount is below the static `estimated_fee`, it must reject
    /// pre-broadcast with a typed `AddressOperation` error rather than letting
    /// the chain refuse it with a generic `fee_fully_covered = false`.
    #[tokio::test]
    async fn transfer_with_change_address_rejects_sub_fee_reduce_output_target() {
        let wallet = build_short_circuit_wallet();
        let signer = NullSigner;
        let target = p2pkh(0x77);
        let input_addr = p2pkh(0x11);
        // Target output amount well below `estimate_min_fee(1, 1, _)`
        // (≈6.5M credits): the guard must fire.
        let outputs = outputs_for(target, 100_000);
        let mut inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        inputs.insert(input_addr, 100_000);
        let fee_strategy = FeeStrategyByAddress::reduce_output(target);

        let err = wallet
            .transfer_with_change_address(
                0,
                InputSelection::Explicit(inputs),
                outputs,
                None,
                fee_strategy,
                None,
                &signer,
            )
            .await
            .expect_err("sub-fee ReduceOutput target must be rejected");
        match err {
            PlatformWalletError::AddressOperation(msg) => {
                assert!(
                    msg.contains("cannot absorb estimated fee"),
                    "unexpected message: {msg}"
                );
                assert!(
                    msg.contains("ReduceOutput target at index"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected AddressOperation, got {other:?}"),
        }
    }
}
