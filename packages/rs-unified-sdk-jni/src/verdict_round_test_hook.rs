//! Test-only entry point: deliver one credit-verdict round to a Kotlin
//! `NativePersistenceBridge` through the REAL native persistence path.
//!
//! A verdict round is otherwise produced only by the SPV engine processing
//! a transaction it does not credit, which needs a synced chain. The
//! instrumented `WalletManagerRoundTripTest` instead builds the round's
//! `CoreChangeSet` here and hands it to an `FFIPersister` wired to the
//! bridge exactly as `nativeCreate` wires the manager's
//! ([`build_vtable`] + [`restrict_account_writers`] + [`build_extension`]),
//! so everything downstream of the engine runs for real:
//! `FFIPersister::store` (begin → verdict slot → changeset → end),
//! `build_utxo_credit_verdicts_for_callback`, the JNI trampolines that pack
//! the verdicts and the `utxos_added` entries, and the Kotlin handler that
//! stages and commits the rows. JVM/Robolectric tests reach none of that.
//!
//! Like `nativeVerifyPersistenceBridgeDescriptors`, it ships in the library
//! (the instrumented suite runs against the normal build) and is reachable
//! only through the `internal` Kotlin `WalletManagerNative` object. It
//! writes only into the bridge the caller passes, which already owns its
//! database, so it grants no access the caller lacks.

use crate::persistence::{
    build_extension, build_vtable, restrict_account_writers, KotlinPersistenceCtx,
};
use crate::support::{guard, throw_sdk_exception};
use dashcore::{Address, BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
use jni::objects::{JByteArray, JClass, JIntArray, JLongArray, JObject, JObjectArray, JString};
use jni::sys::jint;
use jni::JNIEnv;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::transaction_record::{
    OutputDetail, OutputRole, TransactionDirection, TransactionRecord,
};
use key_wallet::transaction_checking::transaction_router::TransactionType;
use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
use platform_wallet::changeset::changeset::UtxoCreditVerdict;
use platform_wallet::changeset::{
    CoreChangeSet, PersistenceCapabilities, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_ffi::{FFIPersister, PersistenceExtensionCallbacks};
use std::ffi::c_void;
use std::str::FromStr;

/// No verdict: the engine credited the output.
const CREDITED: i8 = 0;

/// One fixture output: the transaction it belongs to (`tx_tag`), where it
/// pays, how much, and the engine's verdict on it (`CREDITED` or a
/// `UTXO_CREDIT_VERDICT_*` code).
#[derive(Debug, Clone)]
struct FixtureOutput {
    tx_tag: u32,
    address: Address,
    amount: u64,
    verdict: i8,
    spent_at_height: u32,
}

/// Build the round: one BIP44-account-0 `Received` record per distinct
/// `tx_tag` (outputs in input order, so an output's vout is its position
/// within its tag), confirmed at `block_height`, plus the verdict map keyed
/// by the real outpoints. A tag always yields the same txid for the same
/// outputs, so a later round can re-deliver a transaction an earlier round
/// stored.
fn build_round(outputs: &[FixtureOutput], block_height: u32) -> Result<CoreChangeSet, String> {
    let mut tags: Vec<u32> = Vec::new();
    for output in outputs {
        if !tags.contains(&output.tx_tag) {
            tags.push(output.tx_tag);
        }
    }
    let account = AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    };
    let context = TransactionContext::InBlock(BlockInfo::new(
        block_height,
        BlockHash::from_byte_array([0x42; 32]),
        1_700_000_000,
    ));
    let mut core = CoreChangeSet::default();
    for tag in tags {
        let group: Vec<&FixtureOutput> = outputs.iter().filter(|o| o.tx_tag == tag).collect();
        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                // A foreign funding coin, distinct per tag.
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([(tag & 0xFF) as u8 ^ 0xA5; 32]),
                    vout: tag,
                },
                script_sig: ScriptBuf::new(),
                sequence: 0xffff_ffff,
                witness: Witness::new(),
            }],
            output: group
                .iter()
                .map(|o| TxOut {
                    value: o.amount,
                    script_pubkey: o.address.script_pubkey(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let txid = tx.txid();
        let mut details = Vec::with_capacity(group.len());
        let mut net: i64 = 0;
        for (vout, o) in group.iter().enumerate() {
            let vout = vout as u32;
            details.push(OutputDetail {
                index: vout,
                role: OutputRole::Received,
                address: Some(o.address.clone()),
                value: o.amount,
            });
            net = net.saturating_add(o.amount as i64);
            let verdict = match o.verdict {
                CREDITED => None,
                1 => Some(UtxoCreditVerdict::ObservedSpent {
                    height: o.spent_at_height,
                }),
                2 => Some(UtxoCreditVerdict::Doomed),
                3 => Some(UtxoCreditVerdict::Uncredited),
                other => return Err(format!("unknown verdict code {other}")),
            };
            if let Some(verdict) = verdict {
                core.utxo_credit_verdicts
                    .insert(OutPoint { txid, vout }, verdict);
            }
        }
        let record = TransactionRecord::new(
            tx,
            account,
            context.clone(),
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            details,
            net,
        );
        core.records.push(record.clone());
        core.account_records.push(record);
    }
    Ok(core)
}

/// Copy the Java arrays into fixture outputs; every array must be N long.
#[allow(clippy::too_many_arguments)]
fn read_outputs(
    env: &mut JNIEnv,
    tx_tags: &JIntArray,
    addresses: &JObjectArray,
    amounts: &JLongArray,
    verdicts: &JByteArray,
    spent_at_heights: &JIntArray,
) -> Result<Vec<FixtureOutput>, String> {
    let jni_err = |e: jni::errors::Error| e.to_string();
    let n = env.get_array_length(addresses).map_err(jni_err)? as usize;
    let lengths = [
        env.get_array_length(tx_tags).map_err(jni_err)? as usize,
        env.get_array_length(amounts).map_err(jni_err)? as usize,
        env.get_array_length(verdicts).map_err(jni_err)? as usize,
        env.get_array_length(spent_at_heights).map_err(jni_err)? as usize,
    ];
    if lengths.iter().any(|len| *len != n) {
        return Err(format!(
            "fixture arrays disagree on length: {n} vs {lengths:?}"
        ));
    }
    let mut tags = vec![0i32; n];
    env.get_int_array_region(tx_tags, 0, &mut tags)
        .map_err(jni_err)?;
    let mut values = vec![0i64; n];
    env.get_long_array_region(amounts, 0, &mut values)
        .map_err(jni_err)?;
    let mut codes = vec![0i8; n];
    env.get_byte_array_region(verdicts, 0, &mut codes)
        .map_err(jni_err)?;
    let mut heights = vec![0i32; n];
    env.get_int_array_region(spent_at_heights, 0, &mut heights)
        .map_err(jni_err)?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let element = env
            .get_object_array_element(addresses, i as i32)
            .map_err(jni_err)?;
        let text: String = env
            .get_string(&JString::from(element))
            .map_err(jni_err)?
            .into();
        let address = Address::from_str(&text)
            .map_err(|e| format!("bad fixture address {text}: {e}"))?
            .assume_checked();
        out.push(FixtureOutput {
            tx_tag: u32::try_from(tags[i]).map_err(|_| "negative tx tag".to_string())?,
            address,
            amount: u64::try_from(values[i]).map_err(|_| "negative amount".to_string())?,
            verdict: codes[i],
            spent_at_height: u32::try_from(heights[i])
                .map_err(|_| "negative spent-at height".to_string())?,
        });
    }
    Ok(out)
}

/// TEST ONLY — see the module doc. Delivers one round carrying the fixture
/// outputs (`txTags[i]` groups output `i` into a transaction; its vout is
/// its position in that group) and their credit verdicts (`0` = credited,
/// else a `UTXO_CREDIT_VERDICT_*` code) to `bridge` for `walletId`, through
/// a persister wired like the manager's. Throws `DashSDKException` when
/// the fixture is malformed or the round fails.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_WalletManagerNative_nativeTestDeliverUtxoCreditVerdictRound(
    mut env: JNIEnv,
    _class: JClass,
    bridge: JObject,
    wallet_id: JByteArray,
    tx_tags: JIntArray,
    addresses: JObjectArray,
    amounts: JLongArray,
    verdicts: JByteArray,
    spent_at_heights: JIntArray,
    block_height: jint,
) {
    guard(&mut env, (), |env| {
        let fixture = (|| -> Result<(PlatformWalletChangeSet, [u8; 32]), String> {
            let id = env
                .convert_byte_array(&wallet_id)
                .map_err(|e| e.to_string())?;
            let id: [u8; 32] = id
                .try_into()
                .map_err(|_| "walletId must be 32 bytes".to_string())?;
            let outputs = read_outputs(
                env,
                &tx_tags,
                &addresses,
                &amounts,
                &verdicts,
                &spent_at_heights,
            )?;
            let height =
                u32::try_from(block_height).map_err(|_| "negative block height".to_string())?;
            let core = build_round(&outputs, height)?;
            Ok((
                PlatformWalletChangeSet {
                    core: Some(core),
                    ..Default::default()
                },
                id,
            ))
        })();
        let (changeset, id) = match fixture {
            Ok(value) => value,
            Err(message) => {
                throw_sdk_exception(env, 99, &format!("verdict-round fixture: {message}"));
                return;
            }
        };

        let global = match env.new_global_ref(&bridge) {
            Ok(global) => global,
            Err(_) => {
                throw_sdk_exception(env, 99, "NewGlobalRef(persistence bridge) failed");
                return;
            }
        };
        // Wired exactly as `nativeCreate` wires the manager's persister.
        // The persister owns the context box and frees it (release_fn) on
        // drop, at the end of this call.
        let ctx = Box::into_raw(Box::new(KotlinPersistenceCtx::new(global)));
        let mut vtable = build_vtable(ctx as *mut c_void);
        restrict_account_writers(env, &bridge, &mut vtable);
        let extension = build_extension(env, &bridge);
        let persister = FFIPersister::new_with_persistence_capabilities_and_extensions(
            vtable,
            PersistenceCapabilities::NONE,
            PersistenceExtensionCallbacks {
                dpns_name_states: extension.on_persist_dpns_name_states_fn,
                wallet_changeset_sweeps: extension.on_persist_wallet_changeset_sweeps_fn,
                wallet_changeset_chain_lock_height: extension
                    .on_persist_wallet_changeset_chain_lock_height_fn,
                wallet_changeset_utxo_verdicts: extension
                    .on_persist_wallet_changeset_utxo_verdicts_fn,
                wallet_dashpay_backfill: extension.on_persist_wallet_dashpay_backfill_fn,
                ..Default::default()
            },
        );
        let result = persister.store(id, changeset);
        drop(persister);
        if let Err(e) = result {
            throw_sdk_exception(env, 99, &format!("verdict round failed: {e}"));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(index: u32) -> Address {
        let hash = dashcore::PubkeyHash::from_byte_array([index as u8; 20]);
        Address::new(
            dashcore::Network::Testnet,
            dashcore::address::Payload::PubkeyHash(hash),
        )
    }

    fn output(tx_tag: u32, index: u32, amount: u64, verdict: i8) -> FixtureOutput {
        FixtureOutput {
            tx_tag,
            address: address(index),
            amount,
            verdict,
            spent_at_height: if verdict == 1 { 900 } else { 0 },
        }
    }

    /// Vouts follow position within a tag, a tag is one record in both the
    /// folded and per-account lists, and the verdict map is keyed by the
    /// real outpoints — credited outputs carry none.
    #[test]
    fn build_round_groups_by_tag_and_keys_verdicts_by_real_outpoint() {
        let core = build_round(
            &[
                output(1, 1, 100, CREDITED),
                output(2, 4, 800, 3),
                output(1, 2, 200, 1),
                output(1, 3, 400, 2),
            ],
            1_000,
        )
        .expect("fixture builds");
        assert_eq!(core.records.len(), 2);
        assert_eq!(core.account_records.len(), 2);
        let first = &core.records[0];
        let second = &core.records[1];
        assert_eq!(
            first
                .transaction
                .output
                .iter()
                .map(|o| o.value)
                .collect::<Vec<_>>(),
            vec![100, 200, 400]
        );
        assert_eq!(
            core.utxo_credit_verdicts
                .get(&OutPoint {
                    txid: first.txid,
                    vout: 1
                })
                .copied(),
            Some(UtxoCreditVerdict::ObservedSpent { height: 900 })
        );
        assert_eq!(
            core.utxo_credit_verdicts
                .get(&OutPoint {
                    txid: first.txid,
                    vout: 2
                })
                .copied(),
            Some(UtxoCreditVerdict::Doomed)
        );
        assert_eq!(
            core.utxo_credit_verdicts
                .get(&OutPoint {
                    txid: second.txid,
                    vout: 0
                })
                .copied(),
            Some(UtxoCreditVerdict::Uncredited)
        );
        assert_eq!(core.utxo_credit_verdicts.len(), 3);

        // Same tag + outputs → same txid, so a later round re-delivers it.
        let again = build_round(&[output(2, 4, 800, CREDITED)], 1_000).expect("fixture builds");
        assert_eq!(again.records[0].txid, second.txid);
    }

    #[test]
    fn build_round_rejects_an_unknown_verdict_code() {
        assert!(build_round(&[output(1, 1, 100, 9)], 1_000).is_err());
    }
}
