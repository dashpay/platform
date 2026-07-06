//! C-compatible types for platform address wallet FFI.

use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// PlatformAddress representation
// ---------------------------------------------------------------------------

/// Fixed-size C-compatible platform address.
///
/// `address_type` mirrors the [`PlatformAddress`] variant discriminant
/// (`0 = P2pkh`, `1 = P2sh`) and is preserved faithfully by the
/// [`From<PlatformAddress>`] direction. The **reverse** direction
/// ([`TryFrom<PlatformAddressFFI>`]) used by the platform-address
/// transfer/withdraw entry points (`parse_outputs`,
/// `parse_explicit_inputs`, `parse_explicit_inputs_with_nonces`) accepts
/// `0` (P2PKH) **only** — see that impl for why. Callers driving those
/// entry points must pass `address_type = 0`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformAddressFFI {
    /// `0 = P2pkh`, `1 = P2sh`.
    ///
    /// NOTE: the platform-address transfer/withdraw surface only honors
    /// `0` on the way **in** (see [`TryFrom<PlatformAddressFFI>`]); `1`
    /// round-trips out of [`From<PlatformAddress>`] but is rejected if
    /// fed back into a transfer/withdraw input or output.
    pub address_type: u8,
    /// 20-byte hash
    pub hash: [u8; 20],
}

impl From<PlatformAddress> for PlatformAddressFFI {
    fn from(addr: PlatformAddress) -> Self {
        match addr {
            PlatformAddress::P2pkh(hash) => Self {
                address_type: 0,
                hash,
            },
            PlatformAddress::P2sh(hash) => Self {
                address_type: 1,
                hash,
            },
        }
    }
}

impl TryFrom<PlatformAddressFFI> for PlatformAddress {
    type Error = &'static str;
    /// Accepts `address_type = 0` (P2PKH) **only**.
    ///
    /// This conversion backs the platform-address transfer/withdraw
    /// inputs and outputs (`parse_explicit_inputs`,
    /// `parse_explicit_inputs_with_nonces`, `parse_outputs`). P2SH
    /// (`address_type = 1`) is intentionally rejected here even though
    /// the [`PlatformAddress`] enum and the consensus transition can
    /// represent it:
    ///
    /// - **Inputs** are spent via `Signer<PlatformAddress>`, whose FFI
    ///   `VTableSigner::sign_create_witness` produces only P2PKH
    ///   witnesses and explicitly errors on P2SH (the iOS
    ///   `KeychainSigner` holds P2PKH key material only). A P2SH input
    ///   cannot be signed on this path.
    /// - **Outputs/recipients** on this surface are always P2PKH in
    ///   practice: the wallet derives P2PKH platform-payment addresses,
    ///   and the Swift transfer UI tags own-wallet and pasted-hash
    ///   recipients as P2PKH.
    ///
    /// Accepting `1` here would only relocate the failure deeper (to the
    /// signer for inputs) without enabling a working P2SH transfer, so
    /// the contract is narrowed to P2PKH and the rejection is specific.
    /// The identity-side siblings (`identity_transfer.rs`,
    /// `identity_registration_with_signer.rs`) accept `1` because there
    /// the address is a pure recipient signed by an *identity* key, never
    /// spent as a `PlatformAddress` — a genuinely different capability.
    fn try_from(ffi: PlatformAddressFFI) -> Result<Self, Self::Error> {
        match ffi.address_type {
            0 => Ok(PlatformAddress::P2pkh(ffi.hash)),
            1 => Err("platform-address transfers/withdrawals support P2PKH \
                      (address_type 0) only; P2SH (address_type 1) cannot be \
                      signed or spent on this surface"),
            _ => Err(
                "invalid address_type (platform-address transfers/withdrawals \
                      accept P2PKH, address_type 0, only)",
            ),
        }
    }
}

impl From<key_wallet::PlatformP2PKHAddress> for PlatformAddressFFI {
    fn from(addr: key_wallet::PlatformP2PKHAddress) -> Self {
        Self {
            address_type: 0,
            hash: addr.to_bytes(),
        }
    }
}

// ---------------------------------------------------------------------------
// Fee strategy
// ---------------------------------------------------------------------------

/// C-compatible fee strategy step.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FeeStrategyStepFFI {
    /// 0 = DeductFromInput, 1 = ReduceOutput
    pub step_type: u8,
    pub index: u16,
}

impl From<FeeStrategyStepFFI> for AddressFundsFeeStrategyStep {
    fn from(step: FeeStrategyStepFFI) -> Self {
        match step.step_type {
            0 => AddressFundsFeeStrategyStep::DeductFromInput(step.index),
            _ => AddressFundsFeeStrategyStep::ReduceOutput(step.index),
        }
    }
}

/// Parse a C array of fee strategy steps into a Rust Vec.
///
/// # Safety
/// `ptr` must point to `count` valid `FeeStrategyStepFFI` elements, or be null if `count == 0`.
pub unsafe fn parse_fee_strategy(
    ptr: *const FeeStrategyStepFFI,
    count: usize,
) -> Vec<AddressFundsFeeStrategyStep> {
    if ptr.is_null() || count == 0 {
        return vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];
    }
    std::slice::from_raw_parts(ptr, count)
        .iter()
        .map(|s| AddressFundsFeeStrategyStep::from(*s))
        .collect()
}

// ---------------------------------------------------------------------------
// Input selection
// ---------------------------------------------------------------------------

/// C-compatible input selection type tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSelectionType {
    Explicit = 0,
    ExplicitWithNonces = 1,
    Auto = 2,
}

/// Explicit input entry (address + balance).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExplicitInputFFI {
    pub address: PlatformAddressFFI,
    pub balance: u64,
}

/// Explicit input entry with nonce.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExplicitInputWithNonceFFI {
    pub address: PlatformAddressFFI,
    pub nonce: u32,
    pub balance: u64,
}

/// Parse explicit inputs into a BTreeMap.
///
/// # Safety
/// `ptr` must point to `count` valid elements.
pub unsafe fn parse_explicit_inputs(
    ptr: *const ExplicitInputFFI,
    count: usize,
) -> Result<BTreeMap<PlatformAddress, Credits>, &'static str> {
    if ptr.is_null() && count > 0 {
        return Err("Null input pointer with non-zero count");
    }
    let mut map = BTreeMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address)?;
            if map.contains_key(&addr) {
                return Err("Duplicate input address");
            }
            map.insert(addr, entry.balance);
        }
    }
    Ok(map)
}

/// Parse explicit inputs with nonces into a BTreeMap.
///
/// # Safety
/// `ptr` must point to `count` valid elements.
pub unsafe fn parse_explicit_inputs_with_nonces(
    ptr: *const ExplicitInputWithNonceFFI,
    count: usize,
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, &'static str> {
    if ptr.is_null() && count > 0 {
        return Err("Null input pointer with non-zero count");
    }
    let mut map = BTreeMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address)?;
            if map.contains_key(&addr) {
                return Err("Duplicate input address");
            }
            map.insert(addr, (entry.nonce, entry.balance));
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// Output types (address + balance, used for outputs and results)
// ---------------------------------------------------------------------------

/// Address with balance entry (used for outputs and balance queries).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AddressBalanceEntryFFI {
    pub address: PlatformAddressFFI,
    pub balance: u64,
    /// Address nonce used for anti-replay. Zero when unknown / unused.
    pub nonce: u32,
    /// DIP-17 account index for persisted platform-address state.
    pub account_index: u32,
    /// DIP-17 derivation index within the account.
    pub address_index: u32,
    /// Platform block height `balance` is current as of — the height pin
    /// (see `AddressFunds::as_of_height` in `dash-sdk`). Meaningful on the
    /// persistence round-trip (persist callback → host storage → load);
    /// pass 0 on request paths that only name outputs/amounts.
    pub as_of_height: u64,
}

/// Parse output entries into the DPP-canonical `BTreeMap`.
///
/// Outputs land on-chain in `AddressFundsTransferTransitionV0` as a
/// `BTreeMap<PlatformAddress, Credits>` keyed in lexicographic order;
/// matching that here keeps the FFI boundary aligned with the public
/// transfer API. Duplicate destination addresses are rejected with an
/// explicit error rather than relying on `BTreeMap`'s last-write-wins
/// behaviour, so Swift/Kotlin callers that send the same address twice
/// get a deterministic `Err`.
///
/// # Safety
/// `ptr` must point to `count` valid elements.
pub unsafe fn parse_outputs(
    ptr: *const AddressBalanceEntryFFI,
    count: usize,
) -> Result<BTreeMap<PlatformAddress, Credits>, &'static str> {
    if ptr.is_null() && count > 0 {
        return Err("Null output pointer with non-zero count");
    }
    let mut map = BTreeMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address)?;
            if map.contains_key(&addr) {
                return Err("Duplicate output address");
            }
            map.insert(addr, entry.balance);
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// Withdrawal preflight result
// ---------------------------------------------------------------------------

/// Result of `platform_address_wallet_preflight_withdrawal`: whether an AUTO
/// withdrawal of a platform-payment account can succeed, and — when it can —
/// the net credits that would be paid out plus the reserved transition fee.
///
/// This is a pure, in-memory projection of the Rust planner
/// ([`platform_wallet::wallet::platform_addresses::WithdrawalPlan`]): the SAME
/// planning phase the real withdraw path executes, so a UI gating its submit
/// button on `can_withdraw` can never enable a withdrawal the spend path then
/// rejects (or vice versa).
///
/// A genuine "can't fund" — every address is dust, or the largest input can't
/// retain the fee while clearing the per-input minimum, or the net falls below
/// `min_withdrawal_amount` — is reported as `can_withdraw = false` (a normal
/// result, **not** an FFI error), with `net_withdrawable` and `estimated_fee`
/// left at `0`. Only a structural failure (bad handle, missing account) is an
/// FFI error. The closing typed reason is surfaced via the
/// `PlatformWalletFFIResult` message on the `false` case so the caller can
/// explain *why* without mirroring protocol constants in Swift.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WithdrawalPreflightFFI {
    /// `true` when the account can fund an AUTO withdrawal at the current
    /// platform version; `false` for any "can't fund" case (the fields below
    /// are then `0`).
    pub can_withdraw: bool,
    /// Net credits the chain would pay out (`Σ withdrawable inputs −
    /// estimated_fee`). Valid only when `can_withdraw == true`; `0` otherwise.
    pub net_withdrawable: u64,
    /// The address-credit-withdrawal transition fee reserved on the fee-source
    /// input, sized from the selected input count and the active fee schedule.
    /// Valid only when `can_withdraw == true`; `0` otherwise.
    pub estimated_fee: u64,
}

// ---------------------------------------------------------------------------
// Funding address entry (for top_up)
// ---------------------------------------------------------------------------

/// Address entry for asset lock funding. Exactly one must have `has_balance = false`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FundingAddressEntryFFI {
    pub address: PlatformAddressFFI,
    /// false = None (the funding recipient)
    pub has_balance: bool,
    /// Only valid if has_balance == true
    pub balance: u64,
}

// ---------------------------------------------------------------------------
// Sync config
// ---------------------------------------------------------------------------

/// C-compatible address sync configuration.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AddressSyncConfigFFI {
    pub min_privacy_count: u64,
    pub max_concurrent_requests: u32,
    pub max_iterations: u32,
    pub full_rescan_after_time_s: u64,
}

impl From<AddressSyncConfigFFI> for dash_sdk::platform::address_sync::AddressSyncConfig {
    fn from(c: AddressSyncConfigFFI) -> Self {
        Self {
            min_privacy_count: c.min_privacy_count,
            max_concurrent_requests: c.max_concurrent_requests as usize,
            max_iterations: c.max_iterations as usize,
            full_rescan_after_time_s: c.full_rescan_after_time_s,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Sync result
// ---------------------------------------------------------------------------

/// Found address entry in sync result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FoundAddressEntryFFI {
    pub index: u32,
    pub address: PlatformAddressFFI,
    pub nonce: u32,
    pub balance: u64,
}

/// Absent address entry in sync result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AbsentAddressEntryFFI {
    pub index: u32,
    pub address: PlatformAddressFFI,
}

/// Sync metrics.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AddressSyncMetricsFFI {
    pub trunk_queries: u32,
    pub branch_queries: u32,
    pub total_elements_seen: u32,
    pub total_proof_bytes: u32,
    pub iterations: u32,
    pub compacted_queries: u32,
    pub recent_queries: u32,
    pub recent_entries_returned: u32,
    pub compacted_entries_returned: u32,
}

/// Single account sync result.
#[repr(C)]
pub struct AddressSyncResultFFI {
    pub found: *mut FoundAddressEntryFFI,
    pub found_count: usize,
    pub absent: *mut AbsentAddressEntryFFI,
    pub absent_count: usize,
    pub checkpoint_height: u64,
    pub new_sync_height: u64,
    pub new_sync_timestamp: u64,
    pub last_known_recent_block: u64,
    pub metrics: AddressSyncMetricsFFI,
}

/// Changeset output.
#[repr(C)]
pub struct PlatformAddressChangeSetFFI {
    pub updated: *mut AddressBalanceEntryFFI,
    pub updated_count: usize,
}

impl PlatformAddressChangeSetFFI {
    /// FFI-safe empty sentinel: a null pointer with a zero count.
    ///
    /// The changeset-producing FFI entry points (transfer / withdraw /
    /// fund-from-asset-lock) publish this into their `out_changeset`
    /// out-param *before* any fallible work, so that an error return leaves
    /// the out-param well-defined. `platform_address_wallet_free_changeset`
    /// reconstructs `Vec::from_raw_parts(updated, updated_count, ..)` whenever
    /// `updated` is non-null, so a caller running symmetric cleanup-on-error
    /// over an uninitialized changeset would otherwise feed stale stack bytes
    /// into a real `Vec::from_raw_parts` — a double-free. The `(null, 0)`
    /// sentinel is skipped by that free path.
    pub fn empty() -> Self {
        Self {
            updated: std::ptr::null_mut(),
            updated_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

impl
    From<
        &dash_sdk::platform::address_sync::AddressSyncResult<
            platform_wallet::PlatformAddressTag,
            key_wallet::PlatformP2PKHAddress,
        >,
    > for AddressSyncResultFFI
{
    fn from(
        result: &dash_sdk::platform::address_sync::AddressSyncResult<
            platform_wallet::PlatformAddressTag,
            key_wallet::PlatformP2PKHAddress,
        >,
    ) -> Self {
        // FFI consumers only care about the derivation index from the
        // tag (the caller already knows which wallet/account is
        // syncing). Flatten the tuple by dropping wallet_id and
        // account_index here.
        let found: Vec<FoundAddressEntryFFI> = result
            .found
            .iter()
            .map(|(&((_, _, index), address), funds)| FoundAddressEntryFFI {
                index,
                address: address.into(),
                nonce: funds.nonce,
                balance: funds.balance,
            })
            .collect();

        let absent: Vec<AbsentAddressEntryFFI> = result
            .absent
            .iter()
            .map(|&((_, _, index), address)| AbsentAddressEntryFFI {
                index,
                address: address.into(),
            })
            .collect();

        let found_count = found.len();
        let absent_count = absent.len();

        let found_ptr = if found.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(found.into_boxed_slice()) as *mut FoundAddressEntryFFI
        };

        let absent_ptr = if absent.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(absent.into_boxed_slice()) as *mut AbsentAddressEntryFFI
        };

        let m = &result.metrics;
        Self {
            found: found_ptr,
            found_count,
            absent: absent_ptr,
            absent_count,
            checkpoint_height: result.checkpoint_height,
            new_sync_height: result.new_sync_height,
            new_sync_timestamp: result.new_sync_timestamp,
            last_known_recent_block: result.last_known_recent_block,
            metrics: AddressSyncMetricsFFI {
                trunk_queries: m.trunk_queries as u32,
                branch_queries: m.branch_queries as u32,
                total_elements_seen: m.total_elements_seen as u32,
                total_proof_bytes: m.total_proof_bytes as u32,
                iterations: m.iterations as u32,
                compacted_queries: m.compacted_queries as u32,
                recent_queries: m.recent_queries as u32,
                recent_entries_returned: m.recent_entries_returned as u32,
                compacted_entries_returned: m.compacted_entries_returned as u32,
            },
        }
    }
}

impl From<&platform_wallet::PlatformAddressChangeSet> for PlatformAddressChangeSetFFI {
    fn from(cs: &platform_wallet::PlatformAddressChangeSet) -> Self {
        let updated: Vec<AddressBalanceEntryFFI> = cs
            .addresses
            .iter()
            .map(|entry| AddressBalanceEntryFFI {
                address: entry.address.into(),
                balance: entry.funds.balance,
                nonce: entry.funds.nonce,
                account_index: entry.account_index,
                address_index: entry.address_index,
                as_of_height: entry.funds.as_of_height,
            })
            .collect();

        let updated_count = updated.len();
        let updated_ptr = if updated.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(updated.into_boxed_slice()) as *mut AddressBalanceEntryFFI
        };

        Self {
            updated: updated_ptr,
            updated_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CMT-003: `parse_outputs` must reject duplicate destination addresses
    /// instead of silently overwriting earlier entries. The diagnostic must
    /// name the offending address.
    #[test]
    fn parse_outputs_rejects_duplicate_destination_address() {
        let dup = PlatformAddressFFI {
            address_type: 0,
            hash: [0xAB; 20],
        };
        let entries = [
            AddressBalanceEntryFFI {
                address: dup,
                balance: 1_000_000,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                as_of_height: 0,
            },
            AddressBalanceEntryFFI {
                address: dup,
                balance: 2_000_000,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                as_of_height: 0,
            },
        ];

        let err = unsafe { parse_outputs(entries.as_ptr(), entries.len()) }
            .expect_err("duplicate output address must be rejected");
        assert_eq!(err, "Duplicate output address");
    }

    /// CMT-003: `parse_explicit_inputs` must reject duplicate input addresses
    /// instead of silently overwriting earlier entries. The diagnostic must
    /// name the offending address.
    #[test]
    fn parse_explicit_inputs_rejects_duplicate_input_address() {
        let dup = PlatformAddressFFI {
            address_type: 0,
            hash: [0xCD; 20],
        };
        let entries = [
            ExplicitInputFFI {
                address: dup,
                balance: 1_000_000,
            },
            ExplicitInputFFI {
                address: dup,
                balance: 2_000_000,
            },
        ];

        let err = unsafe { parse_explicit_inputs(entries.as_ptr(), entries.len()) }
            .expect_err("duplicate input address must be rejected");
        assert_eq!(err, "Duplicate input address");
    }

    /// CMT-003: `parse_explicit_inputs_with_nonces` must reject duplicate
    /// input addresses. Same precondition as `parse_explicit_inputs`; the
    /// nonce field doesn't excuse a collision on the address key.
    #[test]
    fn parse_explicit_inputs_with_nonces_rejects_duplicate_input_address() {
        let dup = PlatformAddressFFI {
            address_type: 0,
            hash: [0xEF; 20],
        };
        let entries = [
            ExplicitInputWithNonceFFI {
                address: dup,
                nonce: 1,
                balance: 1_000_000,
            },
            ExplicitInputWithNonceFFI {
                address: dup,
                nonce: 2,
                balance: 2_000_000,
            },
        ];

        let err = unsafe { parse_explicit_inputs_with_nonces(entries.as_ptr(), entries.len()) }
            .expect_err("duplicate input address must be rejected");
        assert_eq!(err, "Duplicate input address");
    }

    /// The platform-address transfer/withdraw surface accepts P2PKH
    /// (`address_type = 0`) only. P2SH (`address_type = 1`) must be
    /// rejected by the shared `TryFrom` with a P2SH-specific message, and
    /// any other discriminant with the generic invalid-type message —
    /// across all three parse entry points (outputs + both input shapes).
    /// The `From<PlatformAddress>` direction still emits `1` for P2SH, so
    /// the asymmetry is intentional and pinned here.
    #[test]
    fn try_from_accepts_p2pkh_and_rejects_p2sh_and_unknown() {
        const P2SH_MSG: &str = "platform-address transfers/withdrawals support P2PKH \
                                (address_type 0) only; P2SH (address_type 1) cannot be \
                                signed or spent on this surface";
        const UNKNOWN_MSG: &str = "invalid address_type (platform-address transfers/withdrawals \
                                   accept P2PKH, address_type 0, only)";

        // 0 → P2pkh round-trips.
        let p2pkh = PlatformAddressFFI {
            address_type: 0,
            hash: [0x11; 20],
        };
        assert_eq!(
            PlatformAddress::try_from(p2pkh).expect("address_type 0 must be accepted"),
            PlatformAddress::P2pkh([0x11; 20]),
        );

        // 1 → rejected with the P2SH-specific message.
        let p2sh = PlatformAddressFFI {
            address_type: 1,
            hash: [0x22; 20],
        };
        assert_eq!(
            PlatformAddress::try_from(p2sh).expect_err("address_type 1 (P2SH) must be rejected"),
            P2SH_MSG,
        );

        // Anything else → generic invalid-type message.
        let unknown = PlatformAddressFFI {
            address_type: 2,
            hash: [0x33; 20],
        };
        assert_eq!(
            PlatformAddress::try_from(unknown).expect_err("unknown address_type must be rejected"),
            UNKNOWN_MSG,
        );

        // The `From` direction still faithfully emits the P2SH
        // discriminant; only the reverse (transfer/withdraw input) path is
        // narrowed.
        assert_eq!(
            PlatformAddressFFI::from(PlatformAddress::P2sh([0x44; 20])).address_type,
            1,
        );
    }

    /// All three input/output parse helpers funnel through the same
    /// narrowed `TryFrom`, so a P2SH (`address_type = 1`) entry is rejected
    /// with the P2SH-specific diagnostic on every entry point.
    #[test]
    fn parse_helpers_reject_p2sh_address_type() {
        const P2SH_MSG: &str = "platform-address transfers/withdrawals support P2PKH \
                                (address_type 0) only; P2SH (address_type 1) cannot be \
                                signed or spent on this surface";

        let p2sh = PlatformAddressFFI {
            address_type: 1,
            hash: [0xAB; 20],
        };

        let out = [AddressBalanceEntryFFI {
            address: p2sh,
            balance: 1_000_000,
            nonce: 0,
            account_index: 0,
            address_index: 0,
            as_of_height: 0,
        }];
        assert_eq!(
            unsafe { parse_outputs(out.as_ptr(), out.len()) }
                .expect_err("parse_outputs must reject P2SH"),
            P2SH_MSG,
        );

        let inp = [ExplicitInputFFI {
            address: p2sh,
            balance: 1_000_000,
        }];
        assert_eq!(
            unsafe { parse_explicit_inputs(inp.as_ptr(), inp.len()) }
                .expect_err("parse_explicit_inputs must reject P2SH"),
            P2SH_MSG,
        );

        let inp_nonce = [ExplicitInputWithNonceFFI {
            address: p2sh,
            nonce: 1,
            balance: 1_000_000,
        }];
        assert_eq!(
            unsafe { parse_explicit_inputs_with_nonces(inp_nonce.as_ptr(), inp_nonce.len()) }
                .expect_err("parse_explicit_inputs_with_nonces must reject P2SH"),
            P2SH_MSG,
        );
    }

    /// Distinct addresses are accepted and the keys end up in DPP-canonical
    /// (lexicographic) order regardless of the caller's array order.
    #[test]
    fn parse_outputs_yields_lex_order_for_distinct_addresses() {
        // Caller-supplied order is intentionally non-lex (0x22 then 0x11);
        // the BTreeMap return type must canonicalize on the way out.
        let entries = [
            AddressBalanceEntryFFI {
                address: PlatformAddressFFI {
                    address_type: 0,
                    hash: [0x22; 20],
                },
                balance: 2,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                as_of_height: 0,
            },
            AddressBalanceEntryFFI {
                address: PlatformAddressFFI {
                    address_type: 0,
                    hash: [0x11; 20],
                },
                balance: 1,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                as_of_height: 0,
            },
        ];

        let map = unsafe { parse_outputs(entries.as_ptr(), entries.len()) }.expect("parse");
        assert_eq!(map.len(), 2);
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys[0], PlatformAddress::P2pkh([0x11; 20]));
        assert_eq!(keys[1], PlatformAddress::P2pkh([0x22; 20]));
    }
}
