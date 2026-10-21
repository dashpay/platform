mod v0;
mod v1;

// Private helpers are selected by the update_masternode_list generation, except
// `drop_unchanged_operator_keys`, which v0 and v1 share.

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey, SerializationFormat};
use dpp::dashcore::ProTxHash;
use dpp::dashcore_rpc::dashcore_rpc_json::{DMNStateDiff, MasternodeListItem};
use std::collections::BTreeMap;

/// The coinbase payload version Dash Core requires while DEPLOYMENT_V24 is active
/// (`MERKLE_ROOT_ASSETUNLOCKS`). Core requires it exactly at the blocks where V24 and V20 are
/// active and refuses it at every other block; V20 is active on every chain Platform runs on.
const V24_COINBASE_PAYLOAD_VERSION: u16 = 4;

/// Whether `new` is the basic-scheme encoding of the BLS public key that `old` encodes in the
/// legacy scheme. The verdict decides whether a masternode keeps its operator identity, so the
/// encodings it accepts are pinned by tests.
fn is_basic_re_encoding_of_legacy_key(old: &[u8], new: &[u8]) -> bool {
    BlsPublicKey::<Bls12381G2Impl>::from_bytes_with_mode(old, SerializationFormat::Legacy)
        .is_ok_and(|key| key.to_bytes() == new)
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Clears `pub_key_operator` in each entry of `updated_mns` whose reported key is the key
    /// `masternode_list` already holds for that masternode: the same bytes, or the basic-scheme
    /// encoding of the key the stored bytes encode in the legacy scheme.
    ///
    /// Once DEPLOYMENT_V24 is active, Dash Core re-serializes a legacy-scheme operator key in the
    /// basic scheme when it raises a masternode's state version, and reports the new bytes as a
    /// changed key. The key is the same. Platform keeps the stored bytes, which its operator
    /// identity's id is derived from, so the masternode keeps that identity. While V24 is not
    /// active at `core_block_height`, every entry stays as Core reports it, so blocks at earlier
    /// Core heights execute as they always have.
    ///
    /// The stored operator key therefore depends on the list's history: a list rebuilt from
    /// Core's full listing holds Core's bytes instead.
    ///
    /// Core is asked whether V24 is active only when an entry would be cleared, at most once.
    ///
    /// Used by `update_state_masternode_list` v0 and v1, so a change of behaviour here needs new
    /// generations of both.
    fn drop_unchanged_operator_keys(
        &self,
        masternode_list: &BTreeMap<ProTxHash, MasternodeListItem>,
        core_block_height: u32,
        updated_mns: &mut [(ProTxHash, DMNStateDiff)],
    ) -> Result<(), Error> {
        let mut v24_active = false;
        for (pro_tx_hash, state_diff) in updated_mns.iter_mut() {
            let (Some(masternode), Some(new_key)) = (
                masternode_list.get(pro_tx_hash),
                state_diff.pub_key_operator.as_deref(),
            ) else {
                continue;
            };
            let stored_key = masternode.state.pub_key_operator.as_slice();
            if new_key != stored_key && !is_basic_re_encoding_of_legacy_key(stored_key, new_key) {
                continue;
            }
            if !v24_active {
                // Every block at a Core height Platform follows carries a coinbase payload.
                let version = self
                    .core_rpc
                    .get_coinbase_payload_version(core_block_height)?
                    .ok_or_else(|| {
                        ExecutionError::DashCoreBadResponseError(format!(
                            "the coinbase of the block at core height {core_block_height} has no payload"
                        ))
                    })?;
                if version < V24_COINBASE_PAYLOAD_VERSION {
                    return Ok(());
                }
                v24_active = true;
            }
            state_diff.pub_key_operator = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_basic_re_encoding_of_legacy_key, V24_COINBASE_PAYLOAD_VERSION};
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::execution::platform_events::core_based_updates::update_masternode_list::operator_key_encoding_tests::{
        bytes, diff_entry, key_and_negation, pro_tx_hash, recorded_masternodes, MN_L1,
        MN_L1_BASIC_KEY, MN_L1_LEGACY_KEY, MN_L2, MN_L2_BASIC_KEY, MN_L2_LEGACY_KEY, MN_L3,
        MN_L3_NEW_KEY, MN_L4,
    };
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore::ProTxHash;
    use dpp::dashcore_rpc::dashcore_rpc_json::DMNStateDiff;
    use dpp::dashcore_rpc::Error as CoreRpcError;
    use serde_json::json;
    use std::str::FromStr;

    /// An operator key as Dash Core v24.0.0-rc.3 printed it before and after re-encoding it, with
    /// the legacy sign bit clear and set.
    const CORE_RE_ENCODINGS: [(&str, &str); 2] = [
        (MN_L1_LEGACY_KEY, MN_L1_BASIC_KEY),
        (MN_L2_LEGACY_KEY, MN_L2_BASIC_KEY),
    ];

    /// The BLS12-381 base field modulus, big-endian: no field element, so no x coordinate.
    const FIELD_MODULUS: &str = "1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab";

    #[test]
    fn should_recognise_the_basic_encoding_of_a_legacy_key_as_core_prints_it() {
        for (i, (legacy, _)) in CORE_RE_ENCODINGS.iter().enumerate() {
            for (j, (_, basic)) in CORE_RE_ENCODINGS.iter().enumerate() {
                assert_eq!(
                    is_basic_re_encoding_of_legacy_key(&bytes(legacy), &bytes(basic)),
                    i == j,
                    "{legacy} -> {basic}"
                );
            }
        }
        for (legacy, basic) in CORE_RE_ENCODINGS {
            let (legacy, basic) = (bytes(legacy), bytes(basic));
            assert!(!is_basic_re_encoding_of_legacy_key(&basic, &legacy));
            assert!(!is_basic_re_encoding_of_legacy_key(&legacy, &legacy));
            assert!(!is_basic_re_encoding_of_legacy_key(&basic, &basic));
            assert!(!is_basic_re_encoding_of_legacy_key(&legacy, &basic[..47]));
        }
    }

    /// Bytes that are not a legacy-scheme public key never match.
    #[test]
    fn should_not_recognise_bytes_that_are_not_a_legacy_key() {
        let basic = bytes(MN_L1_BASIC_KEY);
        // The null key Core prints for a revoked operator; it reads as a curve point outside the
        // prime-order subgroup.
        let null_key = vec![0u8; 48];
        let mut sign_set_x_zero = vec![0u8; 48];
        sign_set_x_zero[0] = 0x80;
        let mut bit_6_set = bytes(MN_L1_LEGACY_KEY);
        bit_6_set[0] |= 0x40;
        let mut bit_5_set = bytes(MN_L1_LEGACY_KEY);
        bit_5_set[0] |= 0x20;
        let mut too_long = bytes(MN_L1_LEGACY_KEY);
        too_long.push(0);
        for old in [
            null_key,
            sign_set_x_zero,
            bytes(FIELD_MODULUS),
            bit_6_set,
            bit_5_set,
            vec![],
            bytes(MN_L1_LEGACY_KEY)[..47].to_vec(),
            too_long,
        ] {
            for new in [basic.clone(), old.clone()] {
                assert!(
                    !is_basic_re_encoding_of_legacy_key(&old, &new),
                    "{} -> {}",
                    hex::encode(&old),
                    hex::encode(&new)
                );
            }
        }
    }

    /// The point at infinity has one encoding in both schemes, so it matches only itself, where
    /// the bytes are unchanged anyway.
    #[test]
    fn should_recognise_the_point_at_infinity_only_as_itself() {
        let mut infinity = vec![0u8; 48];
        infinity[0] = 0xc0;
        assert!(is_basic_re_encoding_of_legacy_key(&infinity, &infinity));
        assert!(!is_basic_re_encoding_of_legacy_key(
            &infinity,
            &bytes(MN_L1_BASIC_KEY)
        ));
    }

    /// The bytes alone do not tell which scheme a stored key is in: a basic-scheme key whose sign
    /// bit is clear also decodes in the legacy scheme, to its negation. So the predicate holds for
    /// a key and its negation too; whoever holds the secret of one holds the secret of the other.
    #[test]
    fn should_hold_for_a_basic_key_with_a_clear_sign_bit_and_its_negation() {
        let (key, negation) = key_and_negation(7);
        assert_eq!(key[0] & 0x20, 0);
        assert!(is_basic_re_encoding_of_legacy_key(&key, &negation));
        // The negation's sign bit is set, so it does not decode in the legacy scheme.
        assert!(!is_basic_re_encoding_of_legacy_key(&negation, &key));
    }

    /// One entry of each kind, as one Core diff: MN_L1's key re-encoded with a voting address
    /// change, MN_L2's key unchanged with a service change, MN_L3's key replaced and MN_L4's key
    /// revoked.
    fn updated_mns() -> Vec<(ProTxHash, DMNStateDiff)> {
        vec![
            (
                pro_tx_hash(MN_L1),
                diff_entry(json!({
                    "pubKeyOperator": MN_L1_BASIC_KEY,
                    "votingAddress": "yd59MzNMSWf5LdNE9qxZGrcFeR2BzjuqM2",
                })),
            ),
            (
                pro_tx_hash(MN_L2),
                diff_entry(json!({
                    "pubKeyOperator": MN_L2_LEGACY_KEY,
                    "service": "127.0.0.1:30009",
                })),
            ),
            (
                pro_tx_hash(MN_L3),
                diff_entry(json!({ "pubKeyOperator": MN_L3_NEW_KEY })),
            ),
            (
                pro_tx_hash(MN_L4),
                diff_entry(json!({ "pubKeyOperator": "00".repeat(48) })),
            ),
        ]
    }

    /// While V24 is active at the block's Core height, the re-encoded and the unchanged key are
    /// cleared, and nothing else: their other fields, the replaced key and the revocation stay.
    /// Before V24 nothing changes. Core is asked once, about the block's Core height.
    #[test]
    fn should_clear_unchanged_operator_keys_only_while_v24_is_active() {
        let masternodes = recorded_masternodes();
        for (coinbase_payload_version, cleared) in [
            (V24_COINBASE_PAYLOAD_VERSION - 1, false),
            (V24_COINBASE_PAYLOAD_VERSION, true),
        ] {
            let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
            platform
                .core_rpc
                .expect_get_coinbase_payload_version()
                .withf(|core_height| *core_height == 401)
                .times(1)
                .returning(move |_| Ok(Some(coinbase_payload_version)));
            let mut entries = updated_mns();
            platform
                .drop_unchanged_operator_keys(&masternodes, 401, &mut entries)
                .expect("expected to normalize the diff");
            let mut expected = updated_mns();
            if cleared {
                expected[0].1.pub_key_operator = None;
                expected[1].1.pub_key_operator = None;
            }
            assert_eq!(
                entries, expected,
                "coinbase payload version {coinbase_payload_version}"
            );
        }
    }

    /// A Core error, or a coinbase without a payload, fails the update instead of being read as
    /// V24 not being active, and leaves the entries as reported.
    #[test]
    fn should_fail_when_core_cannot_say_whether_v24_is_active() {
        let masternodes = recorded_masternodes();
        for answer in [
            Err(CoreRpcError::UnexpectedStructure("refused".to_string())),
            Ok(None),
        ] {
            let mut platform = TestPlatformBuilder::new().build_with_mock_rpc();
            let expect_none = answer.is_ok();
            platform
                .core_rpc
                .expect_get_coinbase_payload_version()
                .times(1)
                .return_once(move |_| answer);
            let mut entries = updated_mns();
            let result = platform.drop_unchanged_operator_keys(&masternodes, 401, &mut entries);
            if expect_none {
                assert!(matches!(
                    result,
                    Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
                        _
                    )))
                ));
            } else {
                assert!(result.is_err());
            }
            assert_eq!(entries, updated_mns());
        }
    }

    /// A real key change, an entry without a key and an entry for a masternode Platform does not
    /// list are kept as reported, without asking Core.
    #[test]
    fn should_keep_other_entries_without_asking_core() {
        let masternodes = recorded_masternodes();
        let platform = TestPlatformBuilder::new().build_with_mock_rpc();
        let unlisted = ProTxHash::from_str(&"11".repeat(32)).expect("expected a pro tx hash");
        let mut entries = vec![
            (
                pro_tx_hash(MN_L1),
                diff_entry(json!({ "pubKeyOperator": MN_L2_BASIC_KEY })),
            ),
            (
                pro_tx_hash(MN_L2),
                diff_entry(json!({ "service": "127.0.0.1:30009" })),
            ),
            (
                unlisted,
                diff_entry(json!({ "pubKeyOperator": MN_L1_BASIC_KEY })),
            ),
        ];
        let reported = entries.clone();
        platform
            .drop_unchanged_operator_keys(&masternodes, 401, &mut entries)
            .expect("expected to normalize the diff");
        assert_eq!(entries, reported);
    }
}
