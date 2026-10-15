mod v0;
mod v1;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::dashcore::ChainLock;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Verify the chain lock against the chain lock quorums held in the platform state
    ///
    /// # Parameters
    ///
    /// * `round` - The consensus round of the block proposal, used for logging
    /// * `platform_state` - The platform state holding the chain lock quorums
    /// * `chain_lock` - The chain lock to verify
    /// * `platform_version` - The platform version selecting the implementation
    ///
    /// # Returns
    ///
    /// * `Ok(Some(true))` - The chain lock is signed by a quorum Platform holds
    /// * `Ok(Some(false))` - The chain lock is invalid. From version 1 this is returned only when
    ///   Platform holds exactly the quorums active at the signing height; version 0 also returns
    ///   it when Platform might not hold the quorum the chain lock was signed with
    /// * `Ok(None)` - Platform cannot tell on its own, so Core has to decide
    /// * `Err(Error::BLSError)` - The signature could not be deserialized
    /// * `Err(Error::Execution)` - An unknown method version, or no quorums to verify against
    pub fn verify_chain_lock_locally(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<Option<bool>, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_chain_lock
            .verify_chain_lock_locally
        {
            0 => self.verify_chain_lock_locally_v0(
                round,
                platform_state,
                chain_lock,
                platform_version,
            ),
            1 => self.verify_chain_lock_locally_v1(
                round,
                platform_state,
                chain_lock,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "verify_chain_lock_locally".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform_state::PlatformState;
    use crate::test::helpers::chain_lock_quorums::{
        chain_lock_test_platform, core_signer, llmq_test_chain_lock_config, quorums,
        sign_chain_lock, state_with_chain_lock_quorums, QuorumChange, QuorumMinedAfterCoreHeight,
        TestQuorum,
    };
    use dpp::version::PlatformVersion;

    #[test]
    fn should_stop_rejecting_chain_lock_signed_by_newly_mined_quorum_from_protocol_version_14() {
        // Platform committed core height 1106 holding the llmq_test quorums {a, b}; Core mined
        // quorum n at 1114 and signed the next chain lock with it. Protocol versions up to 13
        // keep rejecting such a chain lock locally; 14 and later leave it to Core.
        let platform = chain_lock_test_platform();
        let scenario = QuorumMinedAfterCoreHeight::default();
        let state = scenario.platform_state(&platform.platform.state.load());
        let chain_lock = scenario.chain_lock_signed_by_new_quorum();

        let verify = |platform_version: &PlatformVersion| {
            platform
                .platform
                .verify_chain_lock_locally(0, &state, &chain_lock, platform_version)
                .expect("verification runs")
        };

        assert_eq!(
            verify(PlatformVersion::get(13).expect("protocol version 13 exists")),
            Some(false),
            "protocol version 13 must keep its verdict"
        );
        assert_eq!(
            verify(PlatformVersion::get(14).expect("protocol version 14 exists")),
            None
        );
        assert_eq!(verify(PlatformVersion::latest()), None);
    }

    /// llmq_test quorums, each mined in the mining window (offsets 10 to 18) of its own DKG
    /// interval, two of them active at a time like on a local network.
    struct MinedQuorums(Vec<(u32, TestQuorum)>);

    impl MinedQuorums {
        fn new() -> Self {
            Self(
                [1018, 1050, 1066, 1098, 1114, 1146, 1162]
                    .into_iter()
                    .zip(1u8..)
                    .map(|(mined_at, seed)| (mined_at, TestQuorum::new(seed, 0)))
                    .collect(),
            )
        }

        fn active_at(&self, core_height: u32) -> Vec<&TestQuorum> {
            self.0
                .iter()
                .rev()
                .filter(|(mined_at, _)| *mined_at <= core_height)
                .take(2)
                .map(|(_, quorum)| quorum)
                .collect()
        }

        /// The platform state after committing `core_heights` in order, recording a quorum
        /// change whenever the active quorums differ, as Platform does.
        fn state_after(&self, base: &PlatformState, core_heights: &[u32]) -> PlatformState {
            let mut changes = vec![];
            for pair in core_heights.windows(2) {
                let before: Vec<_> = self.active_at(pair[0]).iter().map(|q| q.hash).collect();
                let after: Vec<_> = self.active_at(pair[1]).iter().map(|q| q.hash).collect();
                if before != after {
                    changes.push(QuorumChange {
                        quorums: quorums(&self.active_at(pair[1])),
                        from_core_height: pair[0],
                        to_core_height: pair[1],
                    });
                }
            }
            state_with_chain_lock_quorums(
                base,
                llmq_test_chain_lock_config(),
                *core_heights.last().expect("at least one core height"),
                quorums(&self.active_at(core_heights[0])),
                changes,
            )
        }
    }

    #[test]
    fn should_only_turn_wrong_local_rejections_into_core_checks() {
        // Version 1 may only leave to Core what version 0 rejected; everything version 0
        // accepted or left to Core must stay the same, so no block version 0 accepted is
        // rejected. Valid chain locks must never be rejected locally by version 1.
        let platform = chain_lock_test_platform();
        let base = platform.platform.state.load();
        let mined = MinedQuorums::new();
        let unrelated = TestQuorum::new(99, 0);
        let v0_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let v1_version = PlatformVersion::latest();

        // Each history ends at Platform's last committed core height. The flag says whether
        // version 0 wrongly rejects a valid chain lock there, so the history cannot silently
        // stop exercising the fix.
        let histories: [(&[u32], bool); 9] = [
            // one block at a time past the quorum mined at 1098, then early in the DKG
            // interval 1104..=1127, before the quorum mined at 1114
            (
                &[1097, 1098, 1099, 1100, 1101, 1102, 1103, 1104, 1105, 1106],
                true,
            ),
            // one jump over the quorums mined at 1098 and 1114, ending right after the second
            (&[1085, 1114], true),
            // the same jump ending later, where Platform holds every quorum it needs
            (&[1085, 1120], false),
            // past the mining window of the interval 1104..=1127
            (&[1100, 1120, 1123], false),
            // two changes, so the previous quorums are known exactly from 1067 to 1097
            (&[1060, 1067, 1097, 1099], false),
            // no previous quorums
            (&[1100], false),
            // inside the mining window of the interval 1128..=1151, before the quorum mined at
            // its last height 1146
            (&[1110, 1140], true),
            (&[1110, 1145], true),
            // inside the mining window after its quorum was mined
            (&[1110, 1116], false),
        ];

        let mut rejected_by_v1 = 0;
        for (history, v0_wrongly_rejects) in histories {
            let state = mined.state_after(&base, history);
            let core_height = *history.last().expect("non empty");
            let mut wrongly_rejected_by_v0 = 0;
            for chain_lock_height in core_height + 1..=core_height + 40 {
                let signing_height = chain_lock_height - 8;
                let valid = core_signer(&mined.active_at(signing_height), chain_lock_height);
                for (signer, is_valid) in [(valid, true), (&unrelated, false)] {
                    let chain_lock = sign_chain_lock(signer, chain_lock_height);
                    let v0 = platform
                        .platform
                        .verify_chain_lock_locally_v0(0, &state, &chain_lock, v0_version)
                        .expect("v0 runs");
                    let v1 = platform
                        .platform
                        .verify_chain_lock_locally_v1(0, &state, &chain_lock, v1_version)
                        .expect("v1 runs");
                    let case = format!(
                        "history {history:?}, chain lock height {chain_lock_height}, valid {is_valid}: v0 {v0:?}, v1 {v1:?}"
                    );

                    match v0 {
                        Some(true) => assert_eq!(v1, Some(true), "{case}"),
                        None => assert_eq!(v1, None, "{case}"),
                        Some(false) => assert_ne!(v1, Some(true), "{case}"),
                    }
                    if is_valid {
                        assert_ne!(v1, Some(false), "{case}");
                        wrongly_rejected_by_v0 += usize::from(v0 == Some(false));
                    }
                    rejected_by_v1 += usize::from(v1 == Some(false));
                }
            }
            assert_eq!(
                wrongly_rejected_by_v0 > 0,
                v0_wrongly_rejects,
                "history {history:?}: version 0 wrongly rejected {wrongly_rejected_by_v0} valid chain locks"
            );
        }

        assert!(
            rejected_by_v1 > 0,
            "the histories must reach version 1's local rejections"
        );
    }
}
