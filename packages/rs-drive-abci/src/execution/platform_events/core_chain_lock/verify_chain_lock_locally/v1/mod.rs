use dpp::bls::Signature;

use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{ChainLock, QuorumSigningRequestId};
use dpp::dashcore_rpc::json::QuorumType;

use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::error::execution::ExecutionError;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetV0Methods;
use dpp::version::drive_abci_versions::drive_abci_method_versions::DkgMiningSchedule;
use dpp::version::PlatformVersion;

const CHAIN_LOCK_REQUEST_ID_PREFIX: &str = "clsig";

const SIGN_OFFSET: u32 = 8;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock against the chain lock quorums Platform holds, without Core.
    /// Returning None here means Platform cannot tell on its own whether the chain lock is
    /// valid, because it might not hold the quorums Core signed it with. `Some(false)` is only
    /// returned when Platform holds exactly the quorums active at the signing height.
    ///
    /// This leaves to Core chain locks that version 0 rejects, so it relies on
    /// `verify_chain_lock_through_core` 1 having Core's `verifychainlock` check their signature
    /// before `submitchainlock`, which skips that check for a height Core already has a chain
    /// lock at or above.
    #[inline(always)]
    pub(super) fn verify_chain_lock_locally_v1(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<Option<bool>, Error> {
        let quorum_set = platform_state.chain_lock_validating_quorums();
        let quorum_config = quorum_set.config();

        // First verify that the signature conforms to a signature

        let signature = match Signature::from_compressed(chain_lock.signature.as_bytes()) {
            Some(signature) => signature,
            None => {
                tracing::error!(
                    ?chain_lock,
                    "chain lock signature was not deserializable: h:{} r:{}",
                    platform_state.last_committed_block_height() + 1,
                    round
                );
                return Err(Error::BLSError(dpp::bls::BlsError::DeserializationError(
                    "chain lock signature was not deserializable".to_string(),
                )));
            }
        };

        // we attempt to verify the chain lock locally
        let chain_lock_height = chain_lock.block_height;

        let window_width = quorum_config.window;

        // The last block in the window where the quorums would be the same
        let last_block_in_window = platform_state.last_committed_core_height()
            - platform_state.last_committed_core_height() % window_width
            + window_width
            - 1;

        let verification_height = chain_lock_height.saturating_sub(SIGN_OFFSET);

        if verification_height > last_block_in_window {
            tracing::debug!(
                ?chain_lock,
                "h:{} r:{} skipped message_digest for chain lock at core height {} is {}, verification height {}, last block in window {}",
                platform_state.last_committed_block_height() + 1,
                round,
                chain_lock.block_height,
                platform_state.last_committed_core_height(),
                verification_height,
                last_block_in_window,
            );
            return Ok(None); // the chain lock is too far in the future or the past to verify locally
        }

        // The quorums Platform holds are the ones active at the last committed core height. They
        // stay active at least until a block in which a new quorum commitment can be mined, which
        // is only inside the DKG mining window. That window ends well before the DKG interval
        // does, so the quorums can change before `last_block_in_window`.
        let dkg_mining_schedules = platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .dkg_mining_schedules
            .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "verify_chain_lock_locally v1 requires core_chain_lock.dkg_mining_schedules",
            )))?;
        let first_possible_quorum_change = first_possible_quorum_change_after(
            dkg_mining_schedules,
            quorum_config.quorum_type,
            platform_state.last_committed_core_height(),
        )?;

        let mut selected_quorums = platform_state
            .chain_lock_validating_quorums()
            .select_quorums(chain_lock_height, verification_height);

        // TODO: We can use chain_lock.request_id()

        // From DIP 8: https://github.com/dashpay/dips/blob/master/dip-0008.md#finalization-of-signed-blocks
        // The request id is SHA256("clsig", blockHeight) and the message hash is the block hash of the previously successful attempt.

        let mut engine = QuorumSigningRequestId::engine();

        engine.input(&[CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u8]);
        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(chain_lock.block_height.to_le_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        tracing::trace!(
            ?chain_lock,
            "request id for chain lock at height {} is {}",
            chain_lock.block_height,
            hex::encode(request_id.as_byte_array())
        );

        // Based on the deterministic masternode list at the given height, a quorum must be selected that was active at the time this block was mined

        let probable_quorums = selected_quorums.next().ok_or_else(|| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "No quorums selected for chain lock signature verification for specified height",
            ))
        })?;

        let Some((quorum_hash, quorum)) = probable_quorums.choose_quorum(request_id.as_ref())
        else {
            return Ok(None);
        };

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

        let mut engine = sha256d::Hash::engine();

        let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
        reversed_quorum_hash.reverse();

        engine.input(&[quorum_config.quorum_type as u8]);
        engine.input(reversed_quorum_hash.as_slice());
        engine.input(request_id.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let mut chain_lock_verified = signature
            .verify(
                &quorum.public_key,
                message_digest.as_byte_array().as_slice(),
            )
            .is_ok();

        tracing::debug!(
            ?chain_lock,
            "h:{} r:{} message_digest for chain lock at core height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}, last committed core height {}, verification height {}, last block in window {}",
            platform_state.last_committed_block_height() + 1,
            round,
            chain_lock.block_height,
            hex::encode(message_digest.as_byte_array()),
            hex::encode(reversed_quorum_hash.as_slice()),
            hex::encode(chain_lock.block_hash.as_byte_array()),
            if chain_lock_verified { "verified"} else {"not verified"},
            platform_state.last_committed_core_height(),
            verification_height,
            last_block_in_window,
        );

        if !chain_lock_verified {
            // We should also check the other quorum, as there could be the situation where the core height wasn't updated every block.
            if let Some(second_to_check_quorums) = selected_quorums.next() {
                let Some((quorum_hash, quorum)) =
                    second_to_check_quorums.choose_quorum(request_id.as_ref())
                else {
                    // we return that we are not able to verify
                    return Ok(None);
                };

                // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

                let mut engine = sha256d::Hash::engine();

                let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
                reversed_quorum_hash.reverse();

                engine.input(&[quorum_config.quorum_type as u8]);
                engine.input(reversed_quorum_hash.as_slice());
                engine.input(request_id.as_byte_array());
                engine.input(chain_lock.block_hash.as_byte_array());

                let message_digest = sha256d::Hash::from_engine(engine);

                chain_lock_verified = signature
                    .verify(
                        &quorum.public_key,
                        message_digest.as_byte_array().as_slice(),
                    )
                    .is_ok();

                tracing::debug!(
                    ?chain_lock,
                    "h:{} r:{} tried second quorums message_digest for chain lock at height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}",
                    platform_state.last_committed_block_height() + 1,
                    round,
                    chain_lock.block_height,
                    hex::encode(message_digest.as_byte_array()),
                    hex::encode(reversed_quorum_hash.as_slice()),
                    hex::encode(chain_lock.block_hash.as_byte_array()),
                    if chain_lock_verified { "verified"} else {"not verified"}
                );
                if !chain_lock_verified {
                    // The signing height is between the two stored quorum sets. If more than one
                    // quorum was mined in between, the quorums active at the signing height are
                    // neither of them.
                    tracing::debug!(
                        "chain lock was invalid for both recent and old chain lock quorums, we should validate through core"
                    );
                    return Ok(None);
                }
            } else if !platform_state
                .chain_lock_validating_quorums()
                .has_previous_past_quorums()
            {
                // we don't have old quorums, this means our node is very new.
                tracing::debug!(
                    "we had no previous quorums locally, we should validate through core",
                );
                return Ok(None);
            } else if !selected_quorums.should_be_verifiable() {
                tracing::debug!(
                    "we were in a situation where it would be possible we didn't have all quorums and we couldn't verify locally, we should validate through core",
                );
                return Ok(None);
            } else if verification_height >= first_possible_quorum_change {
                tracing::debug!(
                    first_possible_quorum_change,
                    "a quorum may have been mined after the last committed core height and signed the chain lock, we should validate through core",
                );
                return Ok(None);
            } else {
                tracing::debug!("chain lock was invalid, and we deemed there was no reason to check old quorums");
            }
        }

        Ok(Some(chain_lock_verified))
    }
}

/// The first core height after `last_committed_core_height` at which a quorum commitment of
/// `quorum_type` can be mined, from Dash Core's DKG mining schedules. Core only changes the active
/// quorums when a commitment is mined, so the quorums active at `last_committed_core_height` stay
/// the active ones up to the block before the returned height.
fn first_possible_quorum_change_after(
    dkg_mining_schedules: &[DkgMiningSchedule],
    quorum_type: QuorumType,
    last_committed_core_height: u32,
) -> Result<u32, Error> {
    let next_height = last_committed_core_height.saturating_add(1);

    let Some(schedule) = dkg_mining_schedules
        .iter()
        .find(|schedule| schedule.quorum_type == quorum_type as u8)
    else {
        // Without the type's DKG schedule, any later block may change the quorums
        return Ok(next_height);
    };
    let offset_in_interval = last_committed_core_height
        .checked_rem(schedule.interval)
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "a DKG mining schedule has an interval of 0",
        )))?;
    let interval_start = last_committed_core_height - offset_in_interval;

    Ok(if offset_in_interval < schedule.mining_window_start {
        interval_start.saturating_add(schedule.mining_window_start)
    } else if offset_in_interval < schedule.mining_window_end {
        next_height
    } else {
        interval_start
            .saturating_add(schedule.interval)
            .saturating_add(schedule.mining_window_start)
    })
}

#[cfg(test)]
mod tests {
    use super::first_possible_quorum_change_after;
    use crate::config::ChainLockConfig;
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::signature_verification_quorum_set::{Quorums, VerificationQuorum};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::chain_lock_quorums::{
        chain_lock_test_platform, core_signer, llmq_test_chain_lock_config, quorums,
        sign_chain_lock, state_with_chain_lock_quorums, QuorumChange, QuorumMinedAfterCoreHeight,
        TestQuorum,
    };
    use crate::test::helpers::setup::TempPlatform;
    use dpp::bls::PublicKey;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock, QuorumHash};
    use dpp::dashcore_rpc::json::QuorumType;
    use dpp::version::PlatformVersion;
    use std::str::FromStr;

    fn verify(
        platform: &TempPlatform<MockCoreRPCLike>,
        state: &PlatformState,
        chain_lock: &ChainLock,
    ) -> Option<bool> {
        platform
            .platform
            .verify_chain_lock_locally_v1(0, state, chain_lock, PlatformVersion::latest())
            .expect("verification runs")
    }

    fn llmq_400_60_chain_lock_config() -> ChainLockConfig {
        ChainLockConfig {
            quorum_type: QuorumType::Llmq400_60,
            quorum_size: 400,
            quorum_window: 288,
            quorum_active_signers: 4,
            quorum_rotation: false,
        }
    }

    #[test]
    fn should_not_reject_chain_lock_signed_by_quorum_mined_after_last_committed_core_height() {
        // The chain lock's signing height is past 1114, where Core mined n, and still inside
        // the DKG interval of Platform's core height 1106, so Core signed it with a quorum
        // Platform does not hold.
        let platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = scenario.chain_lock_signed_by_new_quorum();

        assert_eq!(
            verify(&platform, &state, &chain_lock),
            None,
            "a chain lock signed by a quorum Platform could not hold yet must be left to Core \
             (chain lock height {})",
            chain_lock.block_height
        );
    }

    #[test]
    fn should_not_reject_chain_lock_signed_by_quorum_set_between_two_stored_sets() {
        // Platform moved its core height from 1095 straight to 1125, across two quorum mining
        // heights: q1 at 1098 and q2 at 1122. It stored {a, b} (active at 1095) as previous and
        // {q2, q1} (active at 1125) as current, but the quorums active at 1118..=1121 were
        // {q1, a}, which it never saw. A chain lock signed at those heights can be valid while
        // failing against both stored sets.
        let platform = chain_lock_test_platform();

        let (q1, q2, a, b, chain_lock_height) = (0..=u8::MAX)
            .find_map(|salt| {
                let a = TestQuorum::new(2, salt);
                let b = TestQuorum::new(3, salt);
                let q1 = TestQuorum::new(4, salt);
                let q2 = TestQuorum::new(5, salt);
                (1126..=1129)
                    .find(|height| {
                        let signer = core_signer(&[&q1, &a], *height).hash;
                        signer != core_signer(&[&q2, &q1], *height).hash
                            && signer != core_signer(&[&a, &b], *height).hash
                    })
                    .map(|height| (q1, q2, a, b, height))
            })
            .expect("some salt yields a signer neither stored set would choose");

        let state = state_with_chain_lock_quorums(
            &platform.platform.state.load(),
            llmq_test_chain_lock_config(),
            1125,
            quorums(&[&a, &b]),
            vec![QuorumChange {
                quorums: quorums(&[&q2, &q1]),
                from_core_height: 1095,
                to_core_height: 1125,
            }],
        );
        let signer = core_signer(&[&q1, &a], chain_lock_height);

        assert_eq!(
            verify(
                &platform,
                &state,
                &sign_chain_lock(signer, chain_lock_height)
            ),
            None,
            "a chain lock signed by quorums active between two stored sets must be left to Core \
             (chain lock height {chain_lock_height})"
        );
    }

    #[test]
    fn should_reject_wrong_signer_locally_until_a_quorum_can_be_mined() {
        // From 1106 the next llmq_test commitment can be mined at 1114, so the quorums Platform
        // holds are exactly the ones active at signing heights up to 1113.
        let platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let unrelated = TestQuorum::new(9, 0);

        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1113 + 8)),
            Some(false)
        );
        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1114 + 8)),
            None
        );
    }

    #[test]
    fn should_reject_wrong_signer_locally_up_to_interval_end_once_mining_window_passed() {
        // At 1123 the mining window of the interval 1104..=1127 (1114..=1122) is over, so no
        // quorum can change before the next interval's window at 1138. Signing heights past the
        // interval end are left to Core, as version 0 does.
        let platform = chain_lock_test_platform();
        let (z, a, b, unrelated) = (
            TestQuorum::new(1, 0),
            TestQuorum::new(2, 0),
            TestQuorum::new(3, 0),
            TestQuorum::new(9, 0),
        );
        let state = state_with_chain_lock_quorums(
            &platform.platform.state.load(),
            llmq_test_chain_lock_config(),
            1123,
            quorums(&[&z, &b]),
            vec![QuorumChange {
                quorums: quorums(&[&a, &b]),
                from_core_height: 1113,
                to_core_height: 1115,
            }],
        );

        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1127 + 8)),
            Some(false)
        );
        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1128 + 8)),
            None
        );
    }

    #[test]
    fn should_verify_and_reject_locally_when_signing_height_quorums_are_known() {
        // The signing height is at or below the last committed core height and after the last
        // quorum change, so Platform holds exactly the quorums Core chooses from and decides on
        // its own, without asking Core.
        let platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());

        let chain_lock_height = 1110;
        let signer = core_signer(&[&scenario.a, &scenario.b], chain_lock_height);
        let other = if signer.hash == scenario.a.hash {
            &scenario.b
        } else {
            &scenario.a
        };

        assert_eq!(
            verify(
                &platform,
                &state,
                &sign_chain_lock(signer, chain_lock_height)
            ),
            Some(true)
        );
        assert_eq!(
            verify(
                &platform,
                &state,
                &sign_chain_lock(other, chain_lock_height)
            ),
            Some(false)
        );
    }

    #[test]
    fn should_reject_wrong_signer_in_previous_quorums_only_where_they_are_known() {
        // Platform held {z, y} until 1062, fetched {a, z} at 1067 and held it up to 1068, and
        // holds {b, a} from 1070. It rejects locally against {a, z} only at signing height 1068;
        // 1067, the height it fetched them at, is left to Core. The heights are close together to
        // reach the branch, not to model a network.
        let platform = chain_lock_test_platform();
        let (y, z, a, b, unrelated) = (
            TestQuorum::new(1, 0),
            TestQuorum::new(2, 0),
            TestQuorum::new(3, 0),
            TestQuorum::new(4, 0),
            TestQuorum::new(9, 0),
        );
        let state = state_with_chain_lock_quorums(
            &platform.platform.state.load(),
            llmq_test_chain_lock_config(),
            1070,
            quorums(&[&z, &y]),
            vec![
                QuorumChange {
                    quorums: quorums(&[&a, &z]),
                    from_core_height: 1062,
                    to_core_height: 1067,
                },
                QuorumChange {
                    quorums: quorums(&[&b, &a]),
                    from_core_height: 1068,
                    to_core_height: 1070,
                },
            ],
        );

        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1068 + 8)),
            Some(false)
        );
        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1067 + 8)),
            None
        );
    }

    #[test]
    fn should_leave_chain_lock_to_core_without_previous_quorums() {
        let platform = chain_lock_test_platform();
        let (a, b, unrelated) = (
            TestQuorum::new(2, 0),
            TestQuorum::new(3, 0),
            TestQuorum::new(9, 0),
        );
        let state = state_with_chain_lock_quorums(
            &platform.platform.state.load(),
            llmq_test_chain_lock_config(),
            1106,
            quorums(&[&a, &b]),
            vec![],
        );

        assert_eq!(
            verify(&platform, &state, &sign_chain_lock(&unrelated, 1110)),
            None
        );
    }

    #[test]
    fn should_follow_mainnet_dkg_schedule_when_rejecting_locally() {
        // llmq_400_60 quorums are mined at offsets 20 to 28 of a 288 block interval; the
        // interval starting at 2555424 mines them at 2555444..=2555452. The chain lock is signed
        // for llmq_test, so it is invalid for llmq_400_60 quorums whoever signs it.
        let platform = chain_lock_test_platform();
        let (z, a, b, c, d, unrelated) = (
            TestQuorum::new(1, 0),
            TestQuorum::new(2, 0),
            TestQuorum::new(3, 0),
            TestQuorum::new(4, 0),
            TestQuorum::new(5, 0),
            TestQuorum::new(9, 0),
        );
        let state_at = |core_height: u32| {
            state_with_chain_lock_quorums(
                &platform.platform.state.load(),
                llmq_400_60_chain_lock_config(),
                core_height,
                quorums(&[&z, &a, &b, &c]),
                vec![QuorumChange {
                    quorums: quorums(&[&a, &b, &c, &d]),
                    from_core_height: core_height - 2,
                    to_core_height: core_height - 1,
                }],
            )
        };
        let verify_at = |core_height: u32, signing_height: u32| {
            verify(
                &platform,
                &state_at(core_height),
                &sign_chain_lock(&unrelated, signing_height + 8),
            )
        };

        // before the mining window
        assert_eq!(verify_at(2555430, 2555443), Some(false));
        assert_eq!(verify_at(2555430, 2555444), None);
        // inside it, the next block can carry a commitment
        assert_eq!(verify_at(2555444, 2555445), None);
        assert_eq!(verify_at(2555451, 2555452), None);
        // after its last height, nothing changes until the next interval's window
        assert_eq!(verify_at(2555452, 2555453), Some(false));
    }

    #[test]
    fn should_read_dkg_mining_schedule_from_platform_version() {
        // From 1106 the next llmq_test commitment can be mined at 1114, so a wrong signer at
        // signing height 1113 is rejected locally. Without llmq_test's schedule any later block
        // may change the quorums, so the same chain lock is left to Core.
        let platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = sign_chain_lock(&TestQuorum::new(9, 0), 1113 + 8);
        let verify_with = |platform_version: &PlatformVersion| {
            platform
                .platform
                .verify_chain_lock_locally_v1(0, &state, &chain_lock, platform_version)
        };

        assert_eq!(
            verify_with(PlatformVersion::latest()).expect("verification runs"),
            Some(false)
        );

        let mut without_schedules = PlatformVersion::latest().clone();
        without_schedules
            .drive_abci
            .methods
            .core_chain_lock
            .dkg_mining_schedules = Some(&[]);
        assert_eq!(
            verify_with(&without_schedules).expect("verification runs"),
            None
        );

        without_schedules
            .drive_abci
            .methods
            .core_chain_lock
            .dkg_mining_schedules = None;
        assert!(matches!(
            verify_with(&without_schedules),
            Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
        ));
    }

    #[test]
    fn should_find_first_possible_quorum_change_from_dash_core_dkg_schedule() {
        // Dash Core mines a commitment only at offsets mining_window_start..=mining_window_end
        // of each DKG interval (`llmq/params.h`). The expected schedules are keyed by dashcore's
        // `QuorumType` and the version table by Core's LLMQ type number, so this also checks
        // that numbering.
        let dkg_mining_schedules = PlatformVersion::latest()
            .drive_abci
            .methods
            .core_chain_lock
            .dkg_mining_schedules
            .expect("the latest protocol version has DKG mining schedules");
        let schedules = [
            (QuorumType::LlmqTest, 24, 10, 18),
            (QuorumType::LlmqTestInstantsend, 24, 10, 18),
            (QuorumType::LlmqTestV17, 24, 10, 18),
            (QuorumType::LlmqTestPlatform, 24, 10, 18),
            (QuorumType::LlmqDevnetPlatform, 24, 10, 18),
            (QuorumType::LlmqDevnet, 24, 10, 18),
            (QuorumType::Llmq50_60, 24, 10, 18),
            (QuorumType::Llmq100_67, 24, 10, 18),
            (QuorumType::Llmq25_67, 24, 10, 18),
            (QuorumType::Llmq400_60, 288, 20, 28),
            (QuorumType::Llmq400_85, 576, 20, 48),
        ];
        for (quorum_type, interval, start, end) in schedules {
            let interval_start = interval * 8873;
            let first_change = |core_height| {
                first_possible_quorum_change_after(dkg_mining_schedules, quorum_type, core_height)
                    .expect("the schedule has an interval")
            };
            assert_eq!(
                first_change(interval_start),
                interval_start + start,
                "{quorum_type}"
            );
            assert_eq!(
                first_change(interval_start + start - 1),
                interval_start + start,
                "{quorum_type}"
            );
            assert_eq!(
                first_change(interval_start + start),
                interval_start + start + 1,
                "{quorum_type}"
            );
            assert_eq!(
                first_change(interval_start + end - 1),
                interval_start + end,
                "{quorum_type}"
            );
            assert_eq!(
                first_change(interval_start + end),
                interval_start + interval + start,
                "{quorum_type}"
            );
        }

        // Rotating and unknown types fall back to any later block
        for quorum_type in [
            QuorumType::Llmq60_75,
            QuorumType::LlmqTestDip0024,
            QuorumType::LlmqDevnetDip0024,
            QuorumType::LlmqSingleNode,
        ] {
            assert_eq!(
                first_possible_quorum_change_after(dkg_mining_schedules, quorum_type, 1110)
                    .expect("no schedule to read"),
                1111
            );
        }
    }

    #[test]
    fn should_verify_mainnet_chain_locks_against_quorums_stored_as_core_reports_them() {
        // Mainnet chain locks 2555653 and 2555665, both signed with the llmq_400_60 quorums
        // active at 2555645..=2555657, whose hashes Drive stores in display order. Dash Core
        // signed the first with 0000000000000008726d…6e1b and the second with
        // 000000000000000751333…75bd; choosing by the hashes as displayed would pick
        // 00000000000000057dd2…d873 for the second. The other quorums' keys do not matter for
        // the choice.
        let platform = chain_lock_test_platform();

        let hex_bytes = |hex: &str| hex::decode(hex).expect("hex");
        let public_key =
            |hex: &str| PublicKey::try_from(hex_bytes(hex).as_slice()).expect("public key");
        let (x, y) = (TestQuorum::new(1, 0), TestQuorum::new(2, 0));
        let unrelated_quorums = quorums(&[&x, &y]);
        let mut unrelated_keys = unrelated_quorums
            .iter()
            .map(|(_, quorum)| quorum.public_key);

        let active: Quorums<VerificationQuorum> = [
            ("000000000000000ef024cd737d450c3595da6abfde2f0481a07e39310a8e6ba3", None),
            ("00000000000000057dd284100e0cf0e9477ac8f4d5602534eb1930ea12f0d873", None),
            (
                "0000000000000008726d8f2945f2953f8f530f3e0718a2ebafdb1548e3196e1b",
                Some("b9c07aa758b3267f957991cfaef62d818266fd7ff63579d1706aee637967dfc6a7da48df28407ef652cff21c9292965d"),
            ),
            (
                "000000000000000751333cdbfdb75d4b33200d69abf46451365ea133920175bd",
                Some("996e4c9baf03bf03e167799415a537b71f6724d36ef7068992796cdfa6cd340b7928a093ada63e5d5f061419d74707d5"),
            ),
        ]
        .into_iter()
        .map(|(display_hash, signer_public_key)| {
            (
                QuorumHash::from_byte_array(
                    hex_bytes(display_hash).try_into().expect("32 bytes"),
                ),
                VerificationQuorum {
                    public_key: signer_public_key.map(public_key).unwrap_or_else(|| {
                        unrelated_keys.next().expect("two unrelated keys")
                    }),
                    index: None,
                },
            )
        })
        .collect();

        let state = state_with_chain_lock_quorums(
            &platform.platform.state.load(),
            llmq_400_60_chain_lock_config(),
            2555657,
            unrelated_quorums.clone(),
            vec![QuorumChange {
                quorums: active,
                from_core_height: 2555599,
                to_core_height: 2555600,
            }],
        );

        let chain_lock = |block_height: u32, block_hash: &str, signature: &str| ChainLock {
            block_height,
            block_hash: BlockHash::from_str(block_hash).expect("block hash"),
            signature: <[u8; 96]>::try_from(hex_bytes(signature))
                .expect("96 bytes")
                .into(),
        };
        let first = chain_lock(
            2555653,
            "0000000000000001a26bc13523d40ae79f5680207c0d8486c4939ce2ef01c6c8",
            "83fae388eb55f8e8d9fd89623b5018a1294550f6032b96d5c89de71bf7d567d19cced4023eefcbf9898d77744a2c90860614854ae35d32c0d9268ef06c4859effaf3b9e5f132af58aae0f760cd94ab8b987a6663f01f660d61dbe2ea0856b197",
        );
        let second = chain_lock(
            2555665,
            "000000000000001f346cf0065ba7524b794350270a52fa59d91972182a7513d3",
            "8eedc7f87cc2395b214a0f8dfb7a53c7525f0a5eb16386f2bd30ccbe7666afb7fee8cd4f46416137c0469e1f37d6ad2816c4a05de094276437d5b3d1d64e059294d8ff14acf0ba4a5bb6dc7f649414085d3cccb54db46d89f722ecfa5908a810",
        );

        assert_eq!(verify(&platform, &state, &first), Some(true));
        assert_eq!(verify(&platform, &state, &second), Some(true));

        let mut other_block = first.clone();
        other_block.block_hash = BlockHash::from_byte_array([1u8; 32]);
        assert_eq!(verify(&platform, &state, &other_block), Some(false));
    }
}
