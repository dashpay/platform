//! FFI bindings for claiming a data contract's fee pots on `IdentityWallet`
//! (protocol version 14).
//!
//! Thin: each function reads its C arguments, calls
//! `IdentityWallet::claim_contract_fees_with_signer` or
//! `IdentityWallet::contract_fee_claim_eligibility` on the platform-wallet
//! 8 MB worker stack, and writes the result back as JSON. rs-sdk, the wallet
//! and Drive decide everything else: the signing key, who may claim which
//! pot, whether the pot was already paid out this epoch and how it is split.
//!
//! # The pot argument
//!
//! A pot crosses the boundary as a plain `uint8_t`: 0 for the owner pot, 1
//! for the moderators pot. Any other byte is refused with
//! `ErrorInvalidParameter` before the wallet is looked up, rather than read
//! as a `#[repr(u8)]` enum (an out-of-range value would be undefined
//! behavior), as rs-sdk-ffi's moderation reads take their list byte.
//!
//! # JSON
//!
//! Identifiers are base58 strings, credits and times JSON integers, as the
//! rs-sdk-ffi fee pot read (`dash_sdk_contract_get_fee_pots`) writes them.
//! The pot a claim leaves is written by that read's own writer
//! (`rs_sdk_ffi::contract_fee_pot_json`), so a host decodes it with the type
//! it decodes the read with.

use std::collections::BTreeSet;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use dash_sdk::platform::contract_fee_pots::ContractFeePotState;
use dash_sdk::platform::transition::contract_fee_claim::ClaimedContractFees;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;
use platform_wallet::{ContractFeeClaimEligibility, ModeratorsPotTeam, PlatformWalletError};
use rs_sdk_ffi::{contract_fee_pot_json, SignerHandle, VTableSigner};
use serde_json::{json, Value as JsonValue};

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// The `pot` byte of the owner pot.
pub const CONTRACT_FEE_POT_OWNER: u8 = 0;
/// The `pot` byte of the moderators pot.
pub const CONTRACT_FEE_POT_MODERATORS: u8 = 1;

/// The pot `value` names; any other byte is refused.
pub(crate) fn contract_fee_pot(value: u8) -> Result<ContractFeePot, PlatformWalletFFIResult> {
    match value {
        CONTRACT_FEE_POT_OWNER => Ok(ContractFeePot::Owner),
        CONTRACT_FEE_POT_MODERATORS => Ok(ContractFeePot::Moderators),
        other => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("pot must be 0 (owner) or 1 (moderators), not {other}"),
        )),
    }
}

fn base58(id: &Identifier) -> String {
    id.to_string(Encoding::Base58)
}

/// The name a pot is written with: `"owner"` or `"moderators"`, as the wasm
/// fee claim names it.
fn pot_name(pot: ContractFeePot) -> &'static str {
    match pot {
        ContractFeePot::Owner => "owner",
        ContractFeePot::Moderators => "moderators",
    }
}

/// A claim as its proof shows it: the contract and pot, the pot as the
/// claim left it (its credits and its last claim, in the fee pot read's
/// shape) and the balance after the claim of every identity the proof
/// shows, in identity id order.
pub(crate) fn claimed_contract_fees_json(claimed: &ClaimedContractFees) -> JsonValue {
    let pot_after_claim = ContractFeePotState {
        credits: claimed.remaining_credits,
        last_claim: Some(claimed.last_claim),
    };
    let balances: Vec<JsonValue> = claimed
        .balances
        .iter()
        .map(
            |(identity_id, balance)| json!({"identityId": base58(identity_id), "balance": balance}),
        )
        .collect();
    json!({
        "contractId": base58(&claimed.contract_id),
        "pot": pot_name(claimed.pot),
        "potAfterClaim": contract_fee_pot_json(&pot_after_claim),
        "balances": balances,
    })
}

/// The name a moderators pot team is written with.
fn moderators_pot_team_name(team: ModeratorsPotTeam) -> &'static str {
    match team {
        ModeratorsPotTeam::NoModeration => "noModeration",
        ModeratorsPotTeam::Declared => "declared",
        ModeratorsPotTeam::ElectedInterim => "electedInterim",
        ModeratorsPotTeam::ElectedSeated => "electedSeated",
    }
}

/// Who of the wallet may claim which pot, each list in identity id order.
pub(crate) fn contract_fee_claim_eligibility_json(
    eligibility: &ContractFeeClaimEligibility,
) -> JsonValue {
    let ids = |set: &BTreeSet<Identifier>| -> Vec<String> { set.iter().map(base58).collect() };
    json!({
        "contractId": base58(&eligibility.contract_id),
        "ownerPotClaimants": ids(&eligibility.owner_pot_claimants),
        "moderatorsPotClaimants": ids(&eligibility.moderators_pot_claimants),
        "moderatorsPotTeam": moderators_pot_team_name(eligibility.moderators_pot_team),
    })
}

/// What a host should know after a claim panicked: the panic can strike after the claim was
/// broadcast, so whether the pot was paid out is unknown until it is read again.
const CLAIM_PANIC_GUIDANCE: &str = "The claim may or may not have been broadcast; read the fee \
     pots again before claiming once more.";

/// What a host should know after an eligibility read panicked: a read signs and sends nothing.
const ELIGIBILITY_PANIC_GUIDANCE: &str = "Nothing was signed or broadcast; ask again.";

/// Write `json` to `*out_json`, handing its ownership to the caller.
///
/// # Safety
/// `out_json` must be a valid, writable pointer.
unsafe fn write_json(
    json: JsonValue,
    out_json: *mut *mut c_char,
) -> Result<(), PlatformWalletFFIResult> {
    // A NUL byte inside the JSON would be a serializer bug; refuse it
    // rather than panic across the FFI boundary.
    *out_json = CString::new(json.to_string())?.into_raw();
    Ok(())
}

/// Pay out `pot` of `contract_id`, signed by `claimant_identity_id`, an
/// identity this wallet holds, via the external `signer_handle`.
///
/// Goes through `IdentityWallet::claim_contract_fees_with_signer`: rs-sdk
/// signs with the claimant's first CRITICAL authentication key without
/// contract bounds that the signer holds, fetches the contract again for
/// the proof check, broadcasts on the platform-wallet 8 MB worker stack and
/// waits for the proved pot and balances; the wallet keeps the balance of
/// each of its identities the proof shows. `pot` is 0 for the owner pot and
/// 1 for the moderators pot.
///
/// The proof of a moderators pot claim by a member of an elected contract's
/// seated team shows the claimant's balance alone (rs-sdk's
/// `ClaimedContractFees::balances`), so the other team members this wallet
/// holds keep the balances they had before the claim until they are read
/// again.
///
/// On success writes to `*out_json` the claim as its proof shows it:
/// `{"contractId", "pot": "owner" | "moderators", "potAfterClaim":
/// {"credits", "lastClaimEpoch", "lastClaimTimeMs", "lastClaimantId"},
/// "balances": [{"identityId", "balance"}]}`, where `potAfterClaim` has the
/// shape of one pot of `dash_sdk_contract_get_fee_pots` (its credits are
/// what the split left over plus any fee collected since). Release it with
/// `platform_wallet_string_free`. On any error `*out_json` is left null.
///
/// # Errors
/// - `ErrorInvalidParameter` for a `pot` other than 0 or 1;
/// - `ErrorIdentityNotFound` when the wallet holds no signing identity
///   `claimant_identity_id`;
/// - an SDK error when the signer holds no CRITICAL authentication key
///   without contract bounds of the claimant (eligibility names recipients,
///   not keys, so this reaches the host only here, at claim time);
/// - `ErrorTransactionBroadcastUnconfirmed` for a panic inside, where panics
///   unwind: the claim may or may not have been broadcast;
/// - a refusal by Platform carries its `consensus_code`: a claimant that is
///   not a recipient (`ContractFeeClaimNotAllowedError`), a pot already paid
///   out this epoch (`ContractFeesAlreadyClaimedThisEpochError`, 41111), a
///   pot too small to pay everyone a credit
///   (`ContractFeesNothingToClaimError`).
///
/// # Safety
/// - `claimant_identity_id` and `contract_id` must each point to 32
///   readable bytes.
/// - `signer_handle` must be a live signer, which the caller keeps owning.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contract_fee_claim(
    wallet_handle: Handle,
    claimant_identity_id: *const u8,
    contract_id: *const u8,
    pot: u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_fee_claim",
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        CLAIM_PANIC_GUIDANCE,
        || unsafe {
            contract_fee_claim_inner(
                wallet_handle,
                claimant_identity_id,
                contract_id,
                pot,
                signer_handle,
                out_json,
            )
        },
    )
}

/// The body of [`platform_wallet_contract_fee_claim`], run inside its panic
/// guard.
///
/// # Safety
/// As [`platform_wallet_contract_fee_claim`].
unsafe fn contract_fee_claim_inner(
    wallet_handle: Handle,
    claimant_identity_id: *const u8,
    contract_id: *const u8,
    pot: u8,
    signer_handle: *mut SignerHandle,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // The out slot is checked and nulled before anything else can return,
    // so every error leaves it null.
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(signer_handle);
    check_ptr!(claimant_identity_id);
    check_ptr!(contract_id);

    let claimant_id = unwrap_result_or_return!(read_identifier(claimant_identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));
    let pot = unwrap_result_or_return!(contract_fee_pot(pot));
    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let claimed = identity_wallet
                .claim_contract_fees_with_signer(&claimant_id, &contract_id, pot, signer)
                .await?;
            Ok::<_, PlatformWalletError>(claimed_contract_fees_json(&claimed))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

/// Which of this wallet's signing identities may claim which fee pot of
/// `contract_id`, read through `IdentityWallet::contract_fee_claim_eligibility`
/// on the platform-wallet 8 MB worker stack.
///
/// The owner pot is the contract owner's. The moderators pot is its seated
/// team's (the charter's leader and active members) for an elected contract
/// with a seated charter, and otherwise the team the contract's declaration
/// names. It says who may sign a claim: whether the pot holds enough and
/// whether it was already paid out this epoch are Platform's to refuse at
/// claim time.
///
/// It names recipients, not keys: an identity named here whose signer holds
/// no CRITICAL authentication key without contract bounds is still named,
/// and its claim fails at claim time with that error, which the host shows.
///
/// On success writes to `*out_json` `{"contractId", "ownerPotClaimants":
/// [base58], "moderatorsPotClaimants": [base58], "moderatorsPotTeam":
/// "noModeration" | "declared" | "electedInterim" | "electedSeated"}`;
/// release it with `platform_wallet_string_free`. On any error `*out_json`
/// is left null.
///
/// # Errors
/// - `NotFound` when Platform holds no contract `contract_id`;
/// - a failed read carries the SDK's error;
/// - `ErrorWalletOperation` for a panic inside, where panics unwind.
///
/// # Safety
/// - `contract_id` must point to 32 readable bytes.
/// - `out_json` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contract_fee_claim_eligibility(
    wallet_handle: Handle,
    contract_id: *const u8,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    // `block_on_worker` re-raises a worker's panic here, inside the
    // `extern "C"` frame, where it would abort the host.
    catch_panic_to_code(
        "platform_wallet_contract_fee_claim_eligibility",
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        ELIGIBILITY_PANIC_GUIDANCE,
        || unsafe { contract_fee_claim_eligibility_inner(wallet_handle, contract_id, out_json) },
    )
}

/// The body of [`platform_wallet_contract_fee_claim_eligibility`], run inside
/// its panic guard.
///
/// # Safety
/// As [`platform_wallet_contract_fee_claim_eligibility`].
unsafe fn contract_fee_claim_eligibility_inner(
    wallet_handle: Handle,
    contract_id: *const u8,
    out_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_json);
    *out_json = ptr::null_mut();
    check_ptr!(contract_id);

    let contract_id = unwrap_result_or_return!(read_identifier(contract_id));

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let eligibility = identity_wallet
                .contract_fee_claim_eligibility(&contract_id)
                .await?;
            Ok::<_, PlatformWalletError>(contract_fee_claim_eligibility_json(&eligibility))
        })
    });
    let result = unwrap_option_or_return!(option);
    let json = unwrap_result_or_return!(result);

    unwrap_result_or_return!(write_json(json, out_json));
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dpp::consensus::state::contract_moderation::ContractFeesAlreadyClaimedThisEpochError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::document_type::action_fees::ContractFeePotLastClaim;
    use std::collections::BTreeMap;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    #[test]
    fn should_read_the_pot_byte_and_refuse_any_other() {
        assert_eq!(contract_fee_pot(0).expect("owner"), ContractFeePot::Owner);
        assert_eq!(
            contract_fee_pot(1).expect("moderators"),
            ContractFeePot::Moderators
        );
        for byte in [2, 7, u8::MAX] {
            let refused = contract_fee_pot(byte).expect_err("not a pot");
            assert_eq!(
                refused.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
        }
    }

    /// The pot a claim leaves is written as the fee pot read writes one, so
    /// the host decodes both with one type.
    #[test]
    fn should_write_the_claim_with_the_pot_in_the_fee_pot_read_shape() {
        let last_claim = ContractFeePotLastClaim {
            epoch_index: 12,
            time_ms: 1_700_000_000_000,
            claimant_id: id(3),
        };
        let claimed = ClaimedContractFees {
            contract_id: id(1),
            pot: ContractFeePot::Moderators,
            last_claim,
            remaining_credits: 2,
            balances: BTreeMap::from([(id(4), 900), (id(3), 1_500)]),
        };
        let json = claimed_contract_fees_json(&claimed);
        assert_eq!(
            json,
            json!({
                "contractId": base58(&id(1)),
                "pot": "moderators",
                "potAfterClaim": {
                    "credits": 2,
                    "lastClaimEpoch": 12,
                    "lastClaimTimeMs": 1_700_000_000_000u64,
                    "lastClaimantId": base58(&id(3)),
                },
                "balances": [
                    {"identityId": base58(&id(3)), "balance": 1_500},
                    {"identityId": base58(&id(4)), "balance": 900},
                ],
            })
        );
        assert_eq!(
            json["potAfterClaim"],
            contract_fee_pot_json(&ContractFeePotState {
                credits: 2,
                last_claim: Some(last_claim),
            })
        );
        assert_eq!(
            claimed_contract_fees_json(&ClaimedContractFees {
                pot: ContractFeePot::Owner,
                ..claimed
            })["pot"],
            "owner"
        );
    }

    #[test]
    fn should_write_who_may_claim_each_pot() {
        let eligibility = ContractFeeClaimEligibility {
            contract_id: id(1),
            owner_pot_claimants: BTreeSet::from([id(2)]),
            moderators_pot_claimants: BTreeSet::from([id(5), id(2)]),
            moderators_pot_team: ModeratorsPotTeam::ElectedSeated,
        };
        assert_eq!(
            contract_fee_claim_eligibility_json(&eligibility),
            json!({
                "contractId": base58(&id(1)),
                "ownerPotClaimants": [base58(&id(2))],
                "moderatorsPotClaimants": [base58(&id(2)), base58(&id(5))],
                "moderatorsPotTeam": "electedSeated",
            })
        );
        for (team, name) in [
            (ModeratorsPotTeam::NoModeration, "noModeration"),
            (ModeratorsPotTeam::Declared, "declared"),
            (ModeratorsPotTeam::ElectedInterim, "electedInterim"),
            (ModeratorsPotTeam::ElectedSeated, "electedSeated"),
        ] {
            assert_eq!(moderators_pot_team_name(team), name);
        }
    }

    /// Every pointer and the pot byte are checked before the wallet is
    /// looked up, and the JSON out-param is left null on the way out.
    #[test]
    fn should_refuse_null_pointers_and_a_bad_pot_before_the_wallet_is_looked_up() {
        let id = [0u8; 32];
        let mut out_json: *mut c_char = ptr::null_mut();
        // A dangling but non-null signer: every call below returns before
        // the signer is read.
        let signer = ptr::NonNull::<SignerHandle>::dangling().as_ptr();
        unsafe {
            for result in [
                platform_wallet_contract_fee_claim(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    0,
                    ptr::null_mut(),
                    &mut out_json,
                ),
                platform_wallet_contract_fee_claim(
                    0,
                    id.as_ptr(),
                    id.as_ptr(),
                    0,
                    signer,
                    ptr::null_mut(),
                ),
                platform_wallet_contract_fee_claim_eligibility(0, id.as_ptr(), ptr::null_mut()),
            ] {
                assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            }
            for result in [
                platform_wallet_contract_fee_claim(
                    0,
                    ptr::null(),
                    id.as_ptr(),
                    0,
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_fee_claim(
                    0,
                    id.as_ptr(),
                    ptr::null(),
                    0,
                    signer,
                    &mut out_json,
                ),
                platform_wallet_contract_fee_claim_eligibility(0, ptr::null(), &mut out_json),
            ] {
                assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
                assert!(out_json.is_null());
            }
            let bad_pot = platform_wallet_contract_fee_claim(
                0,
                id.as_ptr(),
                id.as_ptr(),
                2,
                signer,
                &mut out_json,
            );
            assert_eq!(
                bad_pot.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            assert!(out_json.is_null());
        }
    }

    /// The JSON out-param is nulled before the signer is checked, so a null
    /// signer leaves it null as every other error does.
    #[test]
    fn should_null_the_json_out_param_before_refusing_a_null_signer() {
        let id = [0u8; 32];
        // A stale, non-null value the call must overwrite; never read.
        let mut out_json: *mut c_char = ptr::NonNull::<c_char>::dangling().as_ptr();
        let result = unsafe {
            platform_wallet_contract_fee_claim(
                0,
                id.as_ptr(),
                id.as_ptr(),
                0,
                ptr::null_mut(),
                &mut out_json,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(out_json.is_null());
    }

    /// Platform refuses a pot already paid out this epoch; the host gets
    /// the refusal's consensus code beside the result code.
    #[test]
    fn should_carry_the_consensus_code_of_a_pot_already_claimed_this_epoch() {
        let cause = ConsensusError::from(ContractFeesAlreadyClaimedThisEpochError::new(
            id(1),
            ContractFeePot::Owner,
            12,
        ));
        let result: PlatformWalletFFIResult = PlatformWalletError::Sdk(
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 41111,
                message: cause.to_string(),
                cause: Some(cause),
            }),
        )
        .into();
        assert_eq!(result.consensus_code, 41111);
        assert_eq!(
            result.consensus_kind,
            PlatformWalletFFIConsensusErrorKind::State
        );
    }

    #[test]
    fn should_report_a_missing_contract_as_not_found() {
        let result: PlatformWalletFFIResult =
            PlatformWalletError::ContractNotFound { contract_id: id(1) }.into();
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
    }
}
