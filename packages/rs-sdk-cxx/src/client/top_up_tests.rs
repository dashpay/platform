// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! A top-up's funding outcome when Platform answers with Drive's own
//! proofs, signed as a quorum signs them, through the mock SDK's real
//! verification: the wait verifies a snapshot of the identity's balance
//! whether or not the top-up executed, so only the lock's proved record may
//! settle it.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use dash_sdk::dapi_client::transport::TransportRequest;
use dash_sdk::dapi_client::{DumpData, ExecutionResponse};
use dash_sdk::dapi_grpc::mock::Mockable;
use dash_sdk::dapi_grpc::platform::v0::{
    get_path_elements_request, get_path_elements_response,
    wait_for_state_transition_result_response, BroadcastStateTransitionResponse,
    GetPathElementsRequest, GetPathElementsResponse, Proof, ResponseMetadata,
    WaitForStateTransitionResultResponse,
};
use dash_sdk::dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dash_sdk::dpp::block::block_info::BlockInfo;
use dash_sdk::dpp::block::epoch::Epoch;
use dash_sdk::dpp::bls;
use dash_sdk::dpp::dashcore::ecdsa::{EcdsaPublicKey, EcdsaSignature};
use dash_sdk::dpp::dashcore::{OutPoint, Txid};
use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dash_sdk::dpp::key_wallet::signer::{Signer as KeyWalletSigner, SignerMethod};
use dash_sdk::dpp::platform_value::{Bytes32, Bytes36};
use dash_sdk::dpp::prelude::{AssetLockProof, CoreBlockHeight, DataContract};
use dash_sdk::dpp::secp256k1::ecdsa::RecoverableSignature;
use dash_sdk::dpp::secp256k1::Message;
use dash_sdk::dpp::state_transition::identity_topup_transition::methods::IdentityTopUpTransitionMethodsV0;
use dash_sdk::dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dash_sdk::dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
use dash_sdk::platform::ContextProvider;
use drive::drive::{Drive, RootTree};
use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use drive::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use drive::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use drive::util::batch::{DriveOperation, SystemOperationType};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
use tenderdash_abci::signatures::{Hashable, Signable};

use super::*;

/// The quorum whose signature every answer carries.
fn quorum_key() -> bls::SecretKey {
    bls::SecretKey::from_be_bytes(&[0x11; 32]).expect("a BLS secret key")
}

/// Serves the test quorum's key for every quorum.
struct TestQuorum([u8; 48]);

impl ContextProvider for TestQuorum {
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(self.0)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}

/// The asset lock's key, held in-process.
struct LockKey(SecretKey);

#[async_trait]
impl KeyWalletSigner for LockKey {
    type Error = String;

    fn supported_methods(&self) -> &[SignerMethod] {
        &[SignerMethod::Digest]
    }

    async fn sign_ecdsa(
        &self,
        _path: &DerivationPath,
        sighash: [u8; 32],
    ) -> Result<(EcdsaSignature, EcdsaPublicKey), String> {
        let signature =
            RecoverableSignature::sign_ecdsa_recoverable(Message::from_digest(sighash), &self.0);
        Ok((
            signature.to_standard().into(),
            PublicKey::from_secret_key(&self.0).into(),
        ))
    }

    async fn public_key(&self, _path: &DerivationPath) -> Result<EcdsaPublicKey, String> {
        Ok(PublicKey::from_secret_key(&self.0).into())
    }
}

fn lock_key() -> LockKey {
    LockKey(SecretKey::from_secret_bytes([0x42; 32]).expect("a secp256k1 secret key"))
}

/// A directory of mock DAPI answers, removed when dropped.
struct Answers(PathBuf);

impl Answers {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dash-sdk-cxx-top-up-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("an answers directory");
        Answers(dir)
    }

    fn answer<R>(&self, request: &R, response: R::Response)
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
    {
        let answered = Ok(ExecutionResponse {
            inner: response,
            retries: 0,
            address: "https://127.0.0.1:1".parse().expect("an address"),
        });
        let dump = DumpData::new(request, &answered);
        dump.save(&self.0.join(dump.filename().expect("a file name")))
            .expect("the answer is saved");
    }
}

impl Drop for Answers {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `grovedb_proof` of Drive state `root`, at `metadata`, signed by `key` as a
/// quorum signs a precommit.
fn signed(
    key: &bls::SecretKey,
    metadata: &ResponseMetadata,
    root: [u8; 32],
    grovedb_proof: Vec<u8>,
) -> Proof {
    let mut proof = Proof {
        grovedb_proof,
        quorum_hash: vec![2; 32],
        signature: vec![],
        round: 1,
        block_id_hash: vec![3; 32],
        quorum_type: 106,
    };
    let state_id = StateId {
        app_version: metadata.protocol_version as u64,
        core_chain_locked_height: metadata.core_chain_locked_height,
        time: metadata.time_ms,
        app_hash: root.to_vec(),
        height: metadata.height,
    };
    let vote = CanonicalVote {
        r#type: SignedMsgType::Precommit.into(),
        block_id: proof.block_id_hash.clone(),
        chain_id: metadata.chain_id.clone(),
        height: metadata.height as i64,
        round: proof.round as i64,
        state_id: state_id
            .calculate_msg_hash(
                &metadata.chain_id,
                metadata.height as i64,
                proof.round as i32,
            )
            .expect("the signed state hashes"),
    };
    let digest = vote
        .calculate_sign_hash(
            &metadata.chain_id,
            proof.quorum_type as u8,
            &[2; 32],
            metadata.height as i64,
            proof.round as i32,
        )
        .expect("the vote hashes");
    proof.signature = key.sign(&digest).expect("signs").to_bytes().to_vec();
    proof
}

fn record_query(out_point: &OutPoint) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    (
        vec![vec![RootTree::SpentAssetLockTransactions as u8]],
        vec![<[u8; 36]>::from(*out_point).to_vec()],
    )
}

/// What the SDK's top-up returns, what the binding reports for the same
/// top-up, and what `asset_lock_state` reads, when Platform accepts the
/// broadcast and answers the wait and the lock's record from `drive` as it
/// stands.
struct Answered {
    sdk_top_up: Result<u64, Error>,
    binding: (Result<u64, Status>, ffi::FundingReport),
    state: ffi::AssetLockStateResult,
}

fn top_up_answered_by(drive: &Drive, identity: Identifier, proof: &AssetLockProof) -> Answered {
    top_up_answered_with_record(drive, identity, proof, |_| ())
}

/// As [`top_up_answered_by`], with the signed proof of the lock's record
/// passed through `tamper` first.
fn top_up_answered_with_record(
    drive: &Drive,
    identity: Identifier,
    proof: &AssetLockProof,
    tamper: fn(&mut Proof),
) -> Answered {
    let version = PlatformVersion::latest();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("a runtime");
    let topped_up = bare_identity(identity, BTreeMap::new());
    let path = DerivationPath::master();
    let transition: StateTransition = runtime
        .block_on(IdentityTopUpTransition::try_from_identity_with_signer(
            &topped_up,
            proof.clone(),
            &path,
            &lock_key(),
            0,
            version,
            None,
        ))
        .expect("the top-up the SDK builds");

    let metadata = ResponseMetadata {
        height: 10,
        core_chain_locked_height: 100,
        epoch: 0,
        time_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after 1970")
            .as_millis() as u64,
        protocol_version: version.protocol_version,
        chain_id: "dash-sdk-cxx-test".to_string(),
    };
    let root = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .expect("a root hash");
    let key = quorum_key();
    let answers = Answers::new();
    answers.answer(
        &transition
            .broadcast_request_for_state_transition()
            .expect("a broadcast"),
        BroadcastStateTransitionResponse {},
    );
    let wait_proof = drive
        .prove_state_transition(&transition, None, version)
        .expect("Drive proves the top-up's result")
        .into_data()
        .expect("a proof");
    answers.answer(
        &transition
            .wait_for_state_transition_result_request()
            .expect("a wait"),
        WaitForStateTransitionResultResponse {
            version: Some(wait_for_state_transition_result_response::Version::V0(
                wait_for_state_transition_result_response::WaitForStateTransitionResultResponseV0 {
                    result: Some(wait_for_state_transition_result_response::wait_for_state_transition_result_response_v0::Result::Proof(
                        signed(&key, &metadata, root, wait_proof),
                    )),
                    metadata: Some(metadata.clone()),
                },
            )),
        },
    );
    let out_point = proof.out_point().expect("an outpoint");
    let (record_path, record_keys) = record_query(&out_point);
    let record_proof = drive
        .prove_elements(record_path.clone(), record_keys.clone(), None, version)
        .expect("Drive proves the lock's record");
    let mut record_proof = signed(&key, &metadata, root, record_proof);
    tamper(&mut record_proof);
    answers.answer(
        &GetPathElementsRequest {
            version: Some(get_path_elements_request::Version::V0(
                get_path_elements_request::GetPathElementsRequestV0 {
                    path: record_path,
                    keys: record_keys,
                    prove: true,
                },
            )),
        },
        GetPathElementsResponse {
            version: Some(get_path_elements_response::Version::V0(
                get_path_elements_response::GetPathElementsResponseV0 {
                    result: Some(
                        get_path_elements_response::get_path_elements_response_v0::Result::Proof(
                            record_proof,
                        ),
                    ),
                    metadata: Some(metadata),
                },
            )),
        },
    );

    let mut public = [0u8; 48];
    public.copy_from_slice(&key.public_key().to_bytes());
    let mut sdk = SdkBuilder::new_mock()
        .with_version(version)
        .with_context_provider(TestQuorum(public))
        .build()
        .expect("a mock SDK");
    sdk.mock()
        .load_expectations_sync(&answers.0)
        .expect("the answers load");

    runtime.block_on(async {
        let sdk_top_up = topped_up
            .top_up_identity_with_signer(&sdk, proof.clone(), &path, &lock_key(), None)
            .await;
        let built = Mutex::new(None);
        let call = bare_identity(identity, BTreeMap::new())
            .top_up_identity_settled(&sdk, proof.clone(), &path, &lock_key(), 0, |fee| {
                *lock(&built) = Some(fee)
            })
            .await;
        let state = sdk
            .asset_lock_outpoint_state(&out_point)
            .await
            .map_err(Status::from)
            .into();
        Answered {
            sdk_top_up,
            binding: funding_report(Ok(call), 0, &built),
            state,
        }
    })
}

/// The SDK accepts a valid snapshot of the identity's unchanged balance for
/// a top-up that never executed; the binding settles the lock only once
/// Drive's record of it, proved, shows it used up.
#[test]
fn should_settle_a_top_up_only_when_its_lock_is_proved_used_up() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let identity = Identity::random_identity(1, Some(7), version).expect("an identity");
    let before = identity.balance();
    drive
        .add_new_identity(
            identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            version,
        )
        .expect("the identity is stored");
    let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
    let proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point,
    });
    let record = Bytes36::new(out_point.into());
    let lock_value = |remaining| {
        AssetLockValue::new(2_000, vec![0x76], remaining, vec![], version).expect("a lock value")
    };
    let apply = |operations| {
        drive
            .apply_drive_operations(operations, true, &BlockInfo::default(), None, version, None)
            .expect("Drive applies the operations");
    };

    // Not executed: the wait's proof is a genuine, unchanged snapshot.
    let answered = top_up_answered_by(&drive, identity.id(), &proof);
    assert_eq!(answered.sdk_top_up.expect("the SDK verifies it"), before);
    let (result, report) = answered.binding;
    let status = result.expect_err("an unchanged snapshot settles nothing");
    assert_eq!(status.kind, StatusKind::Unavailable);
    assert!(
        status.message.contains("no record of its use"),
        "{status:?}"
    );
    assert_eq!(report.outcome, ffi::FundingOutcome::Submitted);
    assert_eq!(report.last_user_fee_increase, 0);
    assert_eq!(answered.state.status.kind, StatusKind::Ok);
    assert_eq!(answered.state.state, ffi::AssetLockState::NotPresent);

    // A failed transition used part of the lock: still not settled.
    apply(vec![DriveOperation::SystemOperation(
        SystemOperationType::AddUsedAssetLock {
            asset_lock_outpoint: record,
            asset_lock_value: lock_value(1_500),
        },
    )]);
    let answered = top_up_answered_by(&drive, identity.id(), &proof);
    let (result, report) = answered.binding;
    assert!(result
        .unwrap_err()
        .message
        .contains("1500 of its credits are unspent"));
    assert_eq!(report.outcome, ffi::FundingOutcome::Submitted);
    assert_eq!(
        (answered.state.state, answered.state.remaining_credits),
        (ffi::AssetLockState::PartiallyConsumed, 1_500)
    );

    // Executed: Drive's own top-up operations credit the identity and use
    // the lock up.
    let executed = IdentityTopUpTransitionAction::V0(IdentityTopUpTransitionActionV0 {
        signable_bytes_hasher: SignableBytesHasher::PreHashed(Bytes32([9; 32])),
        top_up_asset_lock_value: lock_value(1_500),
        identity_id: identity.id(),
        asset_lock_outpoint: record,
        user_fee_increase: 0,
    });
    apply(
        executed
            .into_high_level_drive_operations(&Epoch::new(0).expect("an epoch"), version)
            .expect("the top-up's operations"),
    );
    let answered = top_up_answered_by(&drive, identity.id(), &proof);
    let (result, report) = answered.binding;
    assert_eq!(result.expect("settled"), before + 1_500);
    assert_eq!(report.outcome, ffi::FundingOutcome::Ok);
    assert_eq!(answered.state.state, ffi::AssetLockState::FullyConsumed);
}

/// A record whose proof fails verification settles nothing, and the top-up
/// says it failed verification, as `asset_lock_state` does, though the lock
/// is used up.
#[test]
fn should_keep_a_failed_record_proof_rejected_and_the_top_up_submitted() {
    let version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let identity = Identity::random_identity(1, Some(7), version).expect("an identity");
    drive
        .add_new_identity(
            identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            version,
        )
        .expect("the identity is stored");
    let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
    let proof = AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point,
    });
    drive
        .apply_drive_operations(
            vec![DriveOperation::SystemOperation(
                SystemOperationType::AddUsedAssetLock {
                    asset_lock_outpoint: Bytes36::new(out_point.into()),
                    asset_lock_value: AssetLockValue::new(2_000, vec![0x76], 0, vec![], version)
                        .expect("a lock value"),
                },
            )],
            true,
            &BlockInfo::default(),
            None,
            version,
            None,
        )
        .expect("Drive applies the operation");
    assert_eq!(
        top_up_answered_by(&drive, identity.id(), &proof)
            .state
            .state,
        ffi::AssetLockState::FullyConsumed
    );

    let answered = top_up_answered_with_record(&drive, identity.id(), &proof, |record| {
        record.signature[0] ^= 1
    });
    assert_eq!(answered.state.status.kind, StatusKind::Rejected);
    let result: ffi::TopUpIdentityResult = answered.binding.into();
    assert_eq!(
        (result.status.kind, result.funding.outcome, result.balance),
        (StatusKind::Rejected, ffi::FundingOutcome::Submitted, 0)
    );
    assert!(
        result.status.message.contains("could not be read"),
        "{:?}",
        result.status
    );
}

/// A lock record that could not be read settles nothing either, and keeps
/// its read's status.
#[test]
fn should_leave_a_top_up_submitted_when_its_lock_record_is_unread() {
    let out_point = OutPoint::new(Txid::from_byte_array([4; 32]), 1);
    for (error, kind) in [
        (
            Error::TimeoutReached(std::time::Duration::from_secs(1), "down".into()),
            StatusKind::Unavailable,
        ),
        (
            Error::InvalidProvedResponse("not a record".into()),
            StatusKind::Rejected,
        ),
    ] {
        let unread = FundingFailure {
            kind: FundingFailureKind::Submitted,
            error: Error::AssetLockNotUsedUp {
                out_point,
                record: UnsettledLockRecord::Unread(Box::new(error)),
            },
        };
        let result: ffi::TopUpIdentityResult =
            funding_report::<u64>(Ok(Err(unread)), 0, &Mutex::new(Some(0))).into();
        assert_eq!(
            (result.status.kind, result.funding.outcome, result.balance),
            (kind, ffi::FundingOutcome::Submitted, 0)
        );
        assert!(
            result.status.message.contains("could not be read"),
            "{:?}",
            result.status
        );
    }
    let failed: ffi::AssetLockStateResult =
        Err::<StoredAssetLockInfo, _>(Status::new(StatusKind::Rejected, "bad proof")).into();
    assert_eq!(
        (failed.status.kind, failed.state),
        (StatusKind::Rejected, ffi::AssetLockState::NotPresent)
    );
}
