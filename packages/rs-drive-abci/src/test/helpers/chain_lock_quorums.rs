//! Chain lock quorums with known keys, and chain locks signed with them the way Dash Core signs
//! them, for testing chain lock verification against crafted platform states.

use crate::config::ChainLockConfig;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSet, SignatureVerificationQuorumSetV0Methods,
    VerificationQuorum,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::bls::SecretKey;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{BlockHash, ChainLock, QuorumHash, QuorumSigningRequestId};
use dpp::dashcore_rpc::json::QuorumType;

/// The chain lock quorum type of local networks (llmq_test): a 24 block DKG interval whose
/// commitment is mined at offsets 10 to 18, with two quorums active.
pub fn llmq_test_chain_lock_config() -> ChainLockConfig {
    ChainLockConfig {
        quorum_type: QuorumType::LlmqTest,
        quorum_size: 3,
        quorum_window: 24,
        quorum_active_signers: 2,
        quorum_rotation: false,
    }
}

/// A chain lock quorum whose threshold secret key is known
pub struct TestQuorum {
    /// The quorum hash
    pub hash: QuorumHash,
    secret_key: SecretKey,
}

impl TestQuorum {
    /// A quorum derived from `seed`; `salt` varies the quorum hash, and with it the quorum's
    /// score for every request id, without changing the key
    pub fn new(seed: u8, salt: u8) -> Self {
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        key_bytes[31] = 1;
        let mut hash_bytes = [seed; 32];
        hash_bytes[0] = salt;
        Self {
            hash: QuorumHash::from_byte_array(hash_bytes),
            secret_key: SecretKey::from_be_bytes(&key_bytes).expect("valid secret key"),
        }
    }

    fn entry(&self) -> (QuorumHash, VerificationQuorum) {
        (
            self.hash,
            VerificationQuorum {
                public_key: self.secret_key.public_key(),
                index: None,
            },
        )
    }
}

/// The verification quorums of `members`
pub fn quorums(members: &[&TestQuorum]) -> Quorums<VerificationQuorum> {
    members.iter().map(|quorum| quorum.entry()).collect()
}

/// The DIP8 request id of a chain lock at `height`
pub fn chain_lock_request_id(height: u32) -> QuorumSigningRequestId {
    let mut engine = QuorumSigningRequestId::engine();
    engine.input(&[5u8]);
    engine.input(b"clsig");
    engine.input(height.to_le_bytes().as_slice());
    QuorumSigningRequestId::from_engine(engine)
}

/// The member of `active_at_signing_height` that Dash Core signs a llmq_test chain lock at
/// `height` with: the DIP8 choice among the quorums active 8 blocks before `height`. It is
/// computed with Platform's own quorum choice, which the mainnet chain lock tests check against
/// quorums Dash Core actually chose.
pub fn core_signer<'q>(active_at_signing_height: &[&'q TestQuorum], height: u32) -> &'q TestQuorum {
    let config = SignatureVerificationQuorumSet::V0(llmq_test_chain_lock_config().into());
    let chosen = quorums(active_at_signing_height)
        .choose_quorum(
            config.config(),
            chain_lock_request_id(height).as_byte_array(),
        )
        .expect("a quorum is chosen")
        .0;
    active_at_signing_height
        .iter()
        .find(|quorum| quorum.hash == chosen)
        .expect("the chosen quorum is one of the active ones")
}

/// A llmq_test chain lock at `height` signed by `signer`
pub fn sign_chain_lock(signer: &TestQuorum, height: u32) -> ChainLock {
    let block_hash = BlockHash::from_byte_array([7u8; 32]);
    let mut reversed_quorum_hash = signer.hash.to_byte_array();
    reversed_quorum_hash.reverse();

    let mut engine = sha256d::Hash::engine();
    engine.input(&[QuorumType::LlmqTest as u8]);
    engine.input(reversed_quorum_hash.as_slice());
    engine.input(chain_lock_request_id(height).as_byte_array());
    engine.input(block_hash.as_byte_array());
    let message_digest = sha256d::Hash::from_engine(engine);

    let signature = signer
        .secret_key
        .sign(message_digest.as_byte_array())
        .expect("signs");

    ChainLock {
        block_height: height,
        block_hash,
        signature: signature.to_bytes().into(),
    }
}

/// A change of the chain lock quorums Platform recorded when its core height moved from
/// `from_core_height` to `to_core_height`
pub struct QuorumChange {
    /// The quorums active at `to_core_height`
    pub quorums: Quorums<VerificationQuorum>,
    /// The last core height Platform had committed with the quorums before the change
    pub from_core_height: u32,
    /// The core height at which Platform fetched `quorums`
    pub to_core_height: u32,
}

/// `base` with last committed core height `core_height` and chain lock quorums of `config` that
/// started as `initial` and then went through `changes`, in order
pub fn state_with_chain_lock_quorums(
    base: &PlatformState,
    config: ChainLockConfig,
    core_height: u32,
    initial: Quorums<VerificationQuorum>,
    changes: Vec<QuorumChange>,
) -> PlatformState {
    let mut state = base.clone();
    state.set_last_committed_block_info(None);
    state.set_genesis_block_info(Some(BlockInfo {
        core_height,
        ..Default::default()
    }));

    let mut quorum_set = SignatureVerificationQuorumSet::V0(config.into());
    quorum_set.set_current_quorums(initial);
    for change in changes {
        quorum_set.replace_quorums(
            change.quorums,
            change.from_core_height,
            change.to_core_height,
        );
    }
    state.set_chain_lock_validating_quorums(quorum_set);
    state
}

/// A platform with a mock Core RPC, for chain lock verification tests
pub fn chain_lock_test_platform() -> TempPlatform<MockCoreRPCLike> {
    TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_initial_state_structure()
}

/// Platform committed core height 1106, early in the llmq_test DKG interval 1104..=1127,
/// holding the quorums {a, b} it fetched at 1091 (before them {z, b}). Core then mines quorum `n`
/// at 1114, the first height of the interval's mining window, and keeps signing chain locks.
pub struct QuorumMinedAfterCoreHeight {
    /// In the quorums before the change at 1091
    pub z: TestQuorum,
    /// In the quorums Platform holds, and still active after `n` is mined
    pub a: TestQuorum,
    /// In the quorums Platform holds
    pub b: TestQuorum,
    /// Mined at 1114, after Platform's core height
    pub n: TestQuorum,
}

impl Default for QuorumMinedAfterCoreHeight {
    fn default() -> Self {
        Self {
            z: TestQuorum::new(1, 0),
            a: TestQuorum::new(2, 0),
            b: TestQuorum::new(3, 0),
            n: TestQuorum::new(4, 0),
        }
    }
}

impl QuorumMinedAfterCoreHeight {
    /// Platform's last committed core height
    pub const CORE_HEIGHT: u32 = 1106;
    /// The height Core mines `n` at
    pub const QUORUM_MINED_AT: u32 = 1114;
    /// The last height of the DKG interval
    pub const INTERVAL_END: u32 = 1127;

    /// `base` holding the chain lock quorums Platform had at core height 1106
    pub fn platform_state(&self, base: &PlatformState) -> PlatformState {
        state_with_chain_lock_quorums(
            base,
            llmq_test_chain_lock_config(),
            Self::CORE_HEIGHT,
            quorums(&[&self.z, &self.b]),
            vec![QuorumChange {
                quorums: quorums(&[&self.a, &self.b]),
                from_core_height: 1089,
                to_core_height: 1091,
            }],
        )
    }

    /// A valid chain lock Core signed with `n`: its signing height is past 1114 and inside the
    /// DKG interval, where Core chooses from {n, a}
    pub fn chain_lock_signed_by_new_quorum(&self) -> ChainLock {
        let height = (Self::QUORUM_MINED_AT + 8..=Self::INTERVAL_END + 8)
            .find(|height| core_signer(&[&self.n, &self.a], *height).hash == self.n.hash)
            .expect("some height in the interval is signed by the new quorum");
        sign_chain_lock(&self.n, height)
    }
}
