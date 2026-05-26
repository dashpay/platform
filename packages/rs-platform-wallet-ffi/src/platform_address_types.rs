//! C-compatible types for platform address wallet FFI.

use dpp::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// PlatformAddress representation
// ---------------------------------------------------------------------------

/// Fixed-size C-compatible platform address.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformAddressFFI {
    /// 0 = P2pkh, 1 = P2sh
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
    fn try_from(ffi: PlatformAddressFFI) -> Result<Self, Self::Error> {
        match ffi.address_type {
            0 => Ok(PlatformAddress::P2pkh(ffi.hash)),
            _ => Err("Unsupported address type"),
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
) -> Result<BTreeMap<PlatformAddress, Credits>, String> {
    if ptr.is_null() && count > 0 {
        return Err("Null input pointer with non-zero count".to_string());
    }
    let mut map = BTreeMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address).map_err(str::to_string)?;
            if map.contains_key(&addr) {
                return Err(format!(
                    "Duplicate input address (hash {})",
                    hex::encode(entry.address.hash)
                ));
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
) -> Result<BTreeMap<PlatformAddress, (AddressNonce, Credits)>, String> {
    if ptr.is_null() && count > 0 {
        return Err("Null input pointer with non-zero count".to_string());
    }
    let mut map = BTreeMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address).map_err(str::to_string)?;
            if map.contains_key(&addr) {
                return Err(format!(
                    "Duplicate input address (hash {})",
                    hex::encode(entry.address.hash)
                ));
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
}

/// Parse output entries into an insertion-ordered `IndexMap`.
///
/// Mirrors `platform-wallet`'s public-API output ordering (QA-002): the
/// wallet preserves the caller's order for UI/debug while DPP still
/// keys the transition by lex-smallest address. Use `IndexMap` here so
/// the caller's array order survives the FFI boundary.
///
/// # Safety
/// `ptr` must point to `count` valid elements.
pub unsafe fn parse_outputs(
    ptr: *const AddressBalanceEntryFFI,
    count: usize,
) -> Result<indexmap::IndexMap<PlatformAddress, Credits>, String> {
    if ptr.is_null() && count > 0 {
        return Err("Null output pointer with non-zero count".to_string());
    }
    let mut map = indexmap::IndexMap::new();
    if count > 0 {
        for entry in std::slice::from_raw_parts(ptr, count) {
            let addr = PlatformAddress::try_from(entry.address).map_err(str::to_string)?;
            if map.contains_key(&addr) {
                return Err(format!(
                    "Duplicate output address (hash {})",
                    hex::encode(entry.address.hash)
                ));
            }
            map.insert(addr, entry.balance);
        }
    }
    Ok(map)
}

// ---------------------------------------------------------------------------
// Funding address entry (for fund_from_asset_lock)
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
            },
            AddressBalanceEntryFFI {
                address: dup,
                balance: 2_000_000,
                nonce: 0,
                account_index: 0,
                address_index: 0,
            },
        ];

        let err = unsafe { parse_outputs(entries.as_ptr(), entries.len()) }
            .expect_err("duplicate output address must be rejected");
        assert!(
            err.contains("Duplicate output address"),
            "unexpected error: {err}"
        );
        // The diagnostic must include the address hash so the caller can
        // identify which output collided.
        assert!(
            err.contains("abababababababababababababababababababab"),
            "address missing from error: {err}"
        );
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
        assert!(err.contains("Duplicate input"), "unexpected error: {err}");
        assert!(
            err.contains("cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"),
            "address missing from error: {err}"
        );
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
        assert!(err.contains("Duplicate input"), "unexpected error: {err}");
        assert!(
            err.contains("efefefefefefefefefefefefefefefefefefefef"),
            "address missing from error: {err}"
        );
    }

    /// Distinct addresses are accepted and the insertion order is preserved.
    #[test]
    fn parse_outputs_preserves_insertion_order_for_distinct_addresses() {
        let entries = [
            AddressBalanceEntryFFI {
                address: PlatformAddressFFI {
                    address_type: 0,
                    hash: [0x11; 20],
                },
                balance: 1,
                nonce: 0,
                account_index: 0,
                address_index: 0,
            },
            AddressBalanceEntryFFI {
                address: PlatformAddressFFI {
                    address_type: 0,
                    hash: [0x22; 20],
                },
                balance: 2,
                nonce: 0,
                account_index: 0,
                address_index: 0,
            },
        ];

        let map = unsafe { parse_outputs(entries.as_ptr(), entries.len()) }.expect("parse");
        assert_eq!(map.len(), 2);
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys[0], PlatformAddress::P2pkh([0x11; 20]));
        assert_eq!(keys[1], PlatformAddress::P2pkh([0x22; 20]));
    }
}
