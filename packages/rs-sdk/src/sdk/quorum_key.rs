//! Verifying a proof signed by a quorum the context provider has not cached.

use super::Sdk;
use crate::Error;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dash_context_provider::{ContextProvider, ContextProviderError};
use drive_proof_verifier::FromProof;
use std::sync::atomic::Ordering;

impl Sdk {
    /// Verify `response` against `provider`, fetching the key of a quorum the
    /// provider has not cached.
    ///
    /// Proof verification is synchronous, so a provider that caches quorum
    /// keys cannot fetch a missing one while it runs. When verification fails
    /// only because the provider has no key for the quorum the proof names,
    /// the provider is asked to fetch it
    /// ([`ContextProvider::fetch_quorum_public_key`]) and the same response is
    /// verified again, signature included. No request is sent again.
    ///
    /// What the provider's trusted source said decides the result:
    /// * the key: the result of the second verification;
    /// * the quorum is absent: the original, retryable error, so the node that
    ///   named the quorum is banned and the request moves on;
    /// * no answer: [`ContextProviderError::QuorumSourceUnavailable`]. It is
    ///   not retryable, so the node is not banned for the source's failure;
    ///   [`crate::sync::retry`] only steps over it briefly to ask another one;
    /// * nothing, because the provider cannot fetch: the original error.
    ///
    /// A verified response's metadata is then accepted
    /// ([`Sdk::accept_verified_metadata`]). One that waited for a fetch is
    /// judged as fresh as it was when it arrived: responses other requests
    /// accepted while it waited do not make it stale.
    pub(crate) async fn verify_fetching_quorum_key<R, O: FromProof<R>>(
        &self,
        request: O::Request,
        response: O::Response,
        provider: &dyn ContextProvider,
        method_name: &'static str,
    ) -> Result<(Option<O>, ResponseMetadata, Proof), Error>
    where
        O::Request: Clone,
        O::Response: Clone,
    {
        // Both verifications use the same protocol version. The request and
        // response are copied on every call so a second verification can
        // follow a fetch; the copy is small next to proof verification itself.
        let version = self.version();
        let seen_height = self.metadata_last_seen_height.load(Ordering::Acquire);
        let missing = match O::maybe_from_proof_with_metadata(
            request.clone(),
            response.clone(),
            self.network,
            version,
            provider,
        ) {
            Ok(verified) => {
                self.accept_verified_metadata(method_name, &verified.1, None)?;
                return Ok(verified);
            }
            Err(error) => error,
        };
        let drive_proof_verifier::Error::QuorumKeyUnavailable {
            quorum_type,
            quorum_hash,
            core_chain_locked_height,
            ..
        } = missing
        else {
            return Err(missing.into());
        };
        let Some(fetch) =
            provider.fetch_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
        else {
            return Err(missing.into());
        };
        let unavailable = |reason: String| {
            tracing::warn!(
                quorum_type,
                quorum_hash = %hex::encode(quorum_hash),
                core_chain_locked_height,
                %reason,
                "could not get the key of the quorum a proof names"
            );
            Err(ContextProviderError::QuorumSourceUnavailable(reason).into())
        };

        match fetch.await {
            Ok(Some(_)) => match O::maybe_from_proof_with_metadata(
                request,
                response,
                self.network,
                version,
                provider,
            ) {
                // The provider broke its contract, for example by not caching
                // the key it returned. The node is not to blame for that.
                Err(drive_proof_verifier::Error::QuorumKeyUnavailable { error, .. }) => {
                    unavailable(format!(
                        "the provider fetched the key but its lookup still misses it: {error}"
                    ))
                }
                Err(error @ drive_proof_verifier::Error::InvalidPublicKey { .. }) => unavailable(
                    format!("the provider supplied an unusable quorum key: {error}"),
                ),
                Ok(verified) => {
                    self.accept_verified_metadata(method_name, &verified.1, Some(seen_height))?;
                    Ok(verified)
                }
                Err(error) => Err(error.into()),
            },
            Ok(None) => {
                tracing::warn!(
                    quorum_type,
                    quorum_hash = %hex::encode(quorum_hash),
                    core_chain_locked_height,
                    "proof names a quorum the trusted quorum source does not list"
                );
                // A fresh trusted answer supersedes the initial lookup's
                // failure: the node named a quorum the source does not know.
                Err(Error::Proof(missing))
            }
            Err(ContextProviderError::QuorumSourceUnavailable(reason)) => unavailable(reason),
            Err(error) => unavailable(error.to_string()),
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use crate::platform::transition::broadcast::{require_execution_proved, WaitForOutcome};
    use crate::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use crate::platform::transition::put_settings::PutSettings;
    use crate::platform::ContextProvider;
    use crate::sdk::SdkInstance;
    use crate::{Error, Sdk, SdkBuilder};
    use dapi_grpc::platform::v0::get_epochs_info_response::{
        get_epochs_info_response_v0::Result as EpochsResult, Version as EpochsVersion,
    };
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
        wait_for_state_transition_result_response_v0::Result as WaitResult, Version as WaitVersion,
        WaitForStateTransitionResultResponseV0,
    };
    use dapi_grpc::platform::v0::{
        GetEpochsInfoRequest, GetEpochsInfoResponse, Proof, ResponseMetadata,
        WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
    };
    use dash_context_provider::{ContextProviderError, QuorumKeyFuture};
    use dpp::block::extended_epoch_info::ExtendedEpochInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use dpp::version::PlatformVersion;
    use rs_dapi_client::mock::MockDapiClient;
    use rs_dapi_client::{
        Address, AddressBanInfo, AddressList, CanRetry, DumpData, ExecutionResponse,
        RequestSettings,
    };
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    /// The quorum that signed the recorded epoch proof, and its public key.
    const SIGNING_QUORUM_HASH: &str =
        "1f0a25d463a2912cd31dd4f91b899a143d6ae990bb9bc89cb34f7c5db8b1b705";

    fn vectors() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/test_epoch_fetch")
    }

    fn signing_quorum_key() -> String {
        std::fs::read_to_string(
            vectors().join(format!("quorum_pubkey-106-{SIGNING_QUORUM_HASH}.json")),
        )
        .expect("read recorded quorum key")
        .trim()
        .to_string()
    }

    /// The recorded request and response of a proved epoch fetch.
    fn recorded_epoch_fetch() -> (GetEpochsInfoRequest, GetEpochsInfoResponse) {
        let (request, response) = DumpData::<GetEpochsInfoRequest>::load(vectors().join(
            "msg_GetEpochsInfoRequest_b2b426ac4a52cb4cb08904c63386caf3663c40a12d3b03827006d66058e439ac.json",
        ))
        .expect("load recorded epoch fetch")
        .deserialize();
        (request, response.expect("recorded response").inner)
    }

    /// Withdrawals authenticate a balance snapshot. The recorded balance
    /// proof therefore exercises their real outcome verifier without
    /// asserting that the withdrawal executed.
    fn recorded_withdrawal_wait() -> (
        StateTransition,
        WaitForStateTransitionResultResponse,
        [u8; 48],
        u64,
    ) {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rs-drive-proof-verifier/tests/vectors/identity-balance");
        let read =
            |file| std::fs::read_to_string(directory.join(file)).expect("read balance vector");
        let manifest: serde_json::Value = serde_json::from_str(&read("manifest.json"))
            .expect("recorded fixture and mock response must be well formed");
        let decode = |file| hex::decode(read(file).trim()).expect("decode recorded proof hex");
        let block = &manifest["block"];
        let metadata = ResponseMetadata {
            height: block["height"]
                .as_u64()
                .expect("integer in vector manifest"),
            core_chain_locked_height: block["core_chain_locked_height"]
                .as_u64()
                .expect("integer in vector manifest")
                .try_into()
                .expect("recorded value fits wire field"),
            epoch: block["epoch"]
                .as_u64()
                .expect("integer in vector manifest")
                .try_into()
                .expect("recorded value fits wire field"),
            time_ms: block["time_ms"]
                .as_u64()
                .expect("integer in vector manifest"),
            protocol_version: block["protocol_version"]
                .as_u64()
                .expect("integer in vector manifest")
                .try_into()
                .expect("recorded value fits wire field"),
            chain_id: block["chain_id"]
                .as_str()
                .expect("string in vector manifest")
                .to_string(),
        };
        let proof_meta = &manifest["proof_meta"];
        let proof = Proof {
            grovedb_proof: decode("proof.hex"),
            signature: decode("signature.hex"),
            quorum_hash: hex::decode(
                proof_meta["quorum_hash_hex"]
                    .as_str()
                    .expect("string in vector manifest"),
            )
            .expect("recorded fixture and mock response must be well formed"),
            block_id_hash: hex::decode(
                proof_meta["block_id_hash_hex"]
                    .as_str()
                    .expect("string in vector manifest"),
            )
            .expect("recorded fixture and mock response must be well formed"),
            round: proof_meta["round"]
                .as_u64()
                .expect("integer in vector manifest")
                .try_into()
                .expect("recorded value fits wire field"),
            quorum_type: proof_meta["quorum_type"]
                .as_u64()
                .expect("integer in vector manifest")
                .try_into()
                .expect("recorded value fits wire field"),
        };
        let identity_id = Identifier::from_bytes(
            &hex::decode(
                manifest["request"]["identity_id"]
                    .as_str()
                    .expect("string in vector manifest"),
            )
            .expect("recorded fixture and mock response must be well formed"),
        )
        .expect("recorded fixture and mock response must be well formed");
        let transition = StateTransition::IdentityCreditWithdrawal(
            IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
                identity_id,
                amount: 100,
                ..Default::default()
            }),
        );
        let response = WaitForStateTransitionResultResponse {
            version: Some(WaitVersion::V0(WaitForStateTransitionResultResponseV0 {
                result: Some(WaitResult::Proof(proof)),
                metadata: Some(metadata),
            })),
        };
        (
            transition,
            response,
            decode("quorum_pubkey.hex")
                .try_into()
                .expect("recorded value fits wire field"),
            manifest["expected"]["balance"]
                .as_u64()
                .expect("integer in vector manifest"),
        )
    }

    struct WaitQuorumSource {
        key: [u8; 48],
        cached: AtomicBool,
        unavailable: bool,
        dapi: Arc<tokio::sync::Mutex<MockDapiClient>>,
        request: WaitForStateTransitionResultRequest,
        failover_response: Option<(WaitForStateTransitionResultResponse, Address)>,
        addresses: AddressList,
        exclusion_seen: Arc<std::sync::Mutex<Option<AddressBanInfo>>>,
    }

    impl ContextProvider for WaitQuorumSource {
        fn get_data_contract(
            &self,
            id: &Identifier,
            version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            CannotFetch.get_data_contract(id, version)
        }

        fn get_token_configuration(
            &self,
            id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            CannotFetch.get_token_configuration(id)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            if self.cached.load(Ordering::SeqCst) {
                if self.failover_response.is_some() {
                    *self.exclusion_seen.lock().expect("exclusion lock") =
                        self.addresses.ban_info().into_iter().find(|ban| ban.banned);
                }
                Ok(self.key)
            } else {
                Err(ContextProviderError::InvalidQuorum(
                    "not cached".to_string(),
                ))
            }
        }

        fn fetch_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Option<QuorumKeyFuture> {
            // Remove the transport response once it has been buffered: a
            // second wait request must fail instead of silently replaying it.
            assert!(self
                .dapi
                .try_lock()
                .expect("transport lock is free during fetch")
                .remove(&self.request));
            if self.unavailable {
                if let Some((response, address)) = &self.failover_response {
                    self.dapi
                        .try_lock()
                        .expect("transport lock is free during fetch")
                        .expect(
                            &self.request,
                            &Ok(ExecutionResponse {
                                inner: response.clone(),
                                address: address.clone(),
                                retries: 0,
                            }),
                        )
                        .expect("configure another node's response");
                    // Another request may fill the cache while this source
                    // lookup fails; the next node can then verify normally.
                    self.cached.store(true, Ordering::SeqCst);
                }
                Some(Box::pin(async {
                    Err(ContextProviderError::QuorumSourceUnavailable(
                        "source offline".to_string(),
                    ))
                }))
            } else {
                self.cached.store(true, Ordering::SeqCst);
                let key = self.key;
                Some(Box::pin(async move { Ok(Some(key)) }))
            }
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    #[derive(Clone, Copy)]
    enum WaitScenario {
        KeyAcquired,
        SourceUnavailable,
        AlreadyStale,
        ForgedSignature,
        SourceFailover,
    }

    /// Key recovery must verify the buffered response even with retries enabled.
    /// Source outages cannot cause a health ban; forged signatures must cause one.
    /// A response stale on arrival remains stale, and source failover uses only
    /// a brief exclusion when another node can be verified.
    #[test_case::test_case(WaitScenario::KeyAcquired; "key_acquired")]
    #[test_case::test_case(WaitScenario::SourceUnavailable; "trusted_source_unavailable")]
    #[test_case::test_case(WaitScenario::AlreadyStale; "already_stale")]
    #[test_case::test_case(WaitScenario::ForgedSignature; "forged_signature_bans")]
    #[test_case::test_case(WaitScenario::SourceFailover; "source_failover_is_brief")]
    #[tokio::test]
    async fn should_verify_the_buffered_broadcast_wait_after_fetching_its_quorum_key(
        scenario: WaitScenario,
    ) {
        let unavailable = matches!(
            scenario,
            WaitScenario::SourceUnavailable | WaitScenario::SourceFailover
        );
        let initially_stale = matches!(scenario, WaitScenario::AlreadyStale);
        let forged_signature = matches!(scenario, WaitScenario::ForgedSignature);
        let failover = matches!(scenario, WaitScenario::SourceFailover);
        let (transition, mut response, key, expected_balance) = recorded_withdrawal_wait();
        let expected_identity = transition.owner_id().expect("withdrawal owner");
        if forged_signature {
            let Some(WaitVersion::V0(v0)) = response.version.as_mut() else {
                panic!("v0 wait");
            };
            let Some(WaitResult::Proof(proof)) = v0.result.as_mut() else {
                panic!("proved wait");
            };
            proof.signature[10] ^= 1;
        }
        let Some(WaitVersion::V0(v0)) = response.version.as_ref() else {
            panic!("v0 wait");
        };
        let metadata = v0.metadata.clone().expect("signed wait metadata");
        let mut sdk = SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .with_version(
                PlatformVersion::get(metadata.protocol_version)
                    .expect("recorded fixture and mock response must be well formed"),
            )
            .with_time_tolerance(None)
            .with_height_tolerance(Some(1))
            .with_trusted_initial_height(metadata.height + if initially_stale { 20 } else { 0 })
            .build()
            .expect("build mock wait SDK");
        let SdkInstance::Mock {
            dapi, address_list, ..
        } = &mut sdk.inner
        else {
            panic!("mock SDK");
        };
        // MockDapiClient reconstructs every response with this address,
        // so bans must be checked against it rather than the supplied metadata.
        let address: Address = "http://127.0.0.1:9000"
            .parse()
            .expect("mock responder address");
        *address_list = address.to_string().parse().expect("valid fixture address");
        let second: Address = "http://127.0.0.1:3002".parse().expect("second address");
        if matches!(
            scenario,
            WaitScenario::KeyAcquired | WaitScenario::SourceFailover
        ) {
            address_list.add(second.clone());
        }
        let failover_response = failover.then(|| (response.clone(), second));
        let request = transition
            .wait_for_state_transition_result_request()
            .expect("construct withdrawal wait request");
        dapi.lock()
            .await
            .expect(
                &request,
                &Ok(ExecutionResponse {
                    inner: response,
                    address: address.clone(),
                    retries: 0,
                }),
            )
            .expect("recorded fixture and mock response must be well formed");
        let exclusion_seen = Arc::new(std::sync::Mutex::new(None));
        let provider = Counting::new(WaitQuorumSource {
            key,
            cached: AtomicBool::new(false),
            unavailable,
            dapi: Arc::clone(dapi),
            request,
            failover_response,
            addresses: address_list.clone(),
            exclusion_seen: Arc::clone(&exclusion_seen),
        });
        if !failover {
            *provider.raise_on_fetch.lock().expect("test state lock") = Some((
                Arc::clone(&sdk.metadata_last_seen_height),
                metadata.height + 10,
            ));
        }
        sdk.set_context_provider(provider.clone());
        let settings = PutSettings {
            request_settings: RequestSettings {
                ban_failed_address: Some(true),
                retries: Some(usize::from(matches!(
                    scenario,
                    WaitScenario::KeyAcquired | WaitScenario::SourceFailover
                ))),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = transition
            .wait_for_outcome_with_metadata(&sdk, Some(settings))
            .await;

        if unavailable && !failover {
            let error = result.unwrap_err();
            assert!(
                matches!(
                    error,
                    Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
                ),
                "got {error:?}"
            );
            assert!(!error.can_retry());
            assert!(!sdk.address_list().is_banned(&address));
            assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        } else if forged_signature {
            let error = result.expect_err("a forged signature cannot verify");
            assert!(
                error.can_retry(),
                "a forged signature is the responding node's fault"
            );
            assert!(matches!(error, Error::Proof(_)), "got {error:?}");
            assert!(
                sdk.address_list().is_banned(&address),
                "banning is enabled for this control case"
            );
            let ban = sdk
                .address_list()
                .ban_info()
                .into_iter()
                .find(|ban| ban.uri == address.to_string())
                .expect("responding node ban");
            assert!(
                ban.banned_until.expect("health ban expiry") - chrono::Utc::now()
                    > chrono::Duration::seconds(30)
            );
            assert_eq!(provider.lookups.load(Ordering::SeqCst), 2);
        } else if initially_stale {
            assert!(matches!(result, Err(Error::StaleNode(_))), "got {result:?}");
            assert_eq!(provider.lookups.load(Ordering::SeqCst), 2);
        } else {
            let (outcome, verified_metadata) =
                result.expect("the buffered wait proof verifies after key acquisition");
            assert_eq!(verified_metadata, metadata);
            assert!(!outcome.is_execution_proved());
            let StateTransitionProofResult::VerifiedPartialIdentity(identity) = outcome.result()
            else {
                panic!("withdrawal balance snapshot");
            };
            assert_eq!(identity.id, expected_identity);
            assert_eq!(identity.balance, Some(expected_balance));
            assert!(matches!(
                require_execution_proved(outcome),
                Err(Error::ExecutionNotProved(_))
            ));
            assert_eq!(provider.lookups.load(Ordering::SeqCst), 2);
        }
        if failover {
            let observed = exclusion_seen.lock().expect("exclusion lock");
            let ban = observed
                .as_ref()
                .expect("first responder excluded before retry verification");
            assert_eq!(ban.uri, address.to_string());
            assert!(ban.banned);
            assert_eq!(
                ban.ban_count, 1,
                "a brief exclusion stays at the ladder floor"
            );
            assert!(
                ban.banned_until.expect("exclusion expiry") - chrono::Utc::now()
                    <= chrono::Duration::seconds(2),
                "source failure must not incur the 60-second health ban"
            );
        }
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        assert_eq!(
            sdk.metadata_last_seen_height.load(Ordering::SeqCst),
            metadata.height
                + if initially_stale {
                    20
                } else if failover {
                    0
                } else {
                    10
                }
        );
    }

    fn with_proof(
        mut response: GetEpochsInfoResponse,
        change: impl FnOnce(&mut dapi_grpc::platform::v0::Proof),
    ) -> GetEpochsInfoResponse {
        let Some(EpochsVersion::V0(v0)) = response.version.as_mut() else {
            panic!("recorded response is v0");
        };
        let Some(EpochsResult::Proof(proof)) = v0.result.as_mut() else {
            panic!("recorded response carries a proof");
        };
        change(proof);
        response
    }

    fn current_list(quorum_hash: &str, key: &str) -> String {
        format!(
            r#"{{"success":true,"data":[{{"quorum_hash":"{quorum_hash}","key":"{key}","height":1,"valid_members_count":3}}]}}"#
        )
    }

    fn empty_previous_list() -> String {
        r#"{"success":true,"data":{"height":1,"quorums":[]}}"#.to_string()
    }

    /// A quorum service that answers each listed path once, then stops. The
    /// returned handle yields the paths it was asked for.
    fn quorum_service(
        responses: Vec<(&'static str, u16, String)>,
    ) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind quorum service");
        listener
            .set_nonblocking(true)
            .expect("make quorum service accept bounded");
        let address = listener.local_addr().expect("quorum service address");
        let mut pending = responses;
        let handle = thread::spawn(move || {
            let mut asked = Vec::new();
            let deadline = Instant::now() + Duration::from_secs(5);
            while !pending.is_empty() && Instant::now() < deadline {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(error) => panic!("accept quorum request: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("make quorum request blocking");
                let path = read_path(&stream);
                let index = pending
                    .iter()
                    .position(|(expected, _, _)| *expected == path)
                    .unwrap_or_else(|| panic!("unexpected quorum request {path}"));
                let (_, status, body) = pending.remove(index);
                write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .expect("answer quorum request");
                asked.push(path);
            }
            asked
        });
        (format!("http://{address}"), handle)
    }

    fn read_path(stream: &TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().expect("clone quorum request"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("read request line");
        let path = line
            .split_whitespace()
            .nth(1)
            .expect("request line has a path")
            .to_string();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("read request header");
            if header == "\r\n" || header.is_empty() {
                return path;
            }
        }
    }

    /// A context provider that counts the lookups and fetches the SDK makes,
    /// and records the quorum it asked to fetch.
    struct Counting {
        inner: Box<dyn ContextProvider>,
        lookups: AtomicUsize,
        fetches: AtomicUsize,
        fetched: std::sync::Mutex<Option<(u32, [u8; 32], u32)>>,
        /// A height mark to raise when the SDK asks for a fetch, as other
        /// requests that complete during the fetch would.
        raise_on_fetch: std::sync::Mutex<Option<(Arc<std::sync::atomic::AtomicU64>, u64)>>,
    }

    impl Counting {
        fn new(inner: impl ContextProvider + 'static) -> Arc<Self> {
            Arc::new(Self {
                inner: Box::new(inner),
                lookups: AtomicUsize::new(0),
                fetches: AtomicUsize::new(0),
                fetched: std::sync::Mutex::new(None),
                raise_on_fetch: std::sync::Mutex::new(None),
            })
        }
    }

    impl ContextProvider for Counting {
        fn get_data_contract(
            &self,
            id: &Identifier,
            platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            self.inner.get_data_contract(id, platform_version)
        }

        fn get_token_configuration(
            &self,
            token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            self.inner.get_token_configuration(token_id)
        }

        fn get_quorum_public_key(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            self.lookups.fetch_add(1, Ordering::SeqCst);
            self.inner
                .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
        }

        fn fetch_quorum_public_key(
            &self,
            quorum_type: u32,
            quorum_hash: [u8; 32],
            core_chain_locked_height: u32,
        ) -> Option<QuorumKeyFuture> {
            self.fetches.fetch_add(1, Ordering::SeqCst);
            *self.fetched.lock().expect("fetched lock") =
                Some((quorum_type, quorum_hash, core_chain_locked_height));
            if let Some((mark, height)) = self.raise_on_fetch.lock().expect("raise lock").as_ref() {
                mark.fetch_max(*height, Ordering::SeqCst);
            }
            self.inner
                .fetch_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            self.inner.get_platform_activation_height()
        }
    }

    /// A trusted provider as wasm builds it: an empty cache, and no blocking
    /// refetch inside the synchronous lookup.
    fn trusted_provider(base_url: String) -> TrustedHttpContextProvider {
        TrustedHttpContextProvider::new_with_url(
            Network::Regtest,
            base_url,
            NonZeroUsize::new(100).expect("non-zero cache size"),
        )
        .expect("trusted provider")
        .with_refetch_if_not_found(false)
    }

    /// A network SDK that verifies proofs with `provider`. It never connects:
    /// the tests hand it recorded responses.
    fn network_sdk(provider: Arc<Counting>) -> Sdk {
        SdkBuilder::new(AddressList::new())
            .with_network(Network::Regtest)
            .with_time_tolerance(None)
            .with_height_tolerance(Some(1))
            .with_trusted_initial_height(2)
            .with_context_provider(provider)
            .build()
            .expect("network sdk")
    }

    async fn verify(
        sdk: &Sdk,
        request: GetEpochsInfoRequest,
        response: GetEpochsInfoResponse,
    ) -> Result<Option<ExtendedEpochInfo>, Error> {
        sdk.parse_proof_with_metadata_and_proof::<GetEpochsInfoRequest, ExtendedEpochInfo>(
            request,
            response,
            "get_epochs_info",
        )
        .await
        .map(|(epoch, _, _)| epoch)
    }

    /// A browser SDK fetched its quorum keys when it connected; once
    /// Platform signs with a newer quorum, every read must still verify,
    /// by fetching that quorum's key and checking the response already
    /// received, instead of failing on every node.
    #[tokio::test]
    async fn should_verify_a_proof_signed_by_a_quorum_the_cache_has_not_seen() {
        let (base_url, service) = quorum_service(vec![
            (
                "/quorums",
                200,
                current_list(SIGNING_QUORUM_HASH, &signing_quorum_key()),
            ),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();
        let core_chain_locked_height = match response.version.as_ref() {
            Some(EpochsVersion::V0(v0)) => {
                v0.metadata
                    .as_ref()
                    .expect("recorded metadata")
                    .core_chain_locked_height
            }
            None => panic!("recorded response is v0"),
        };

        let epoch = verify(&sdk, request, response)
            .await
            .expect("the proof verifies once the signing quorum's key is fetched");

        assert!(epoch.is_some(), "the recorded epoch is proved to exist");
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        let signing_quorum: [u8; 32] = hex::decode(SIGNING_QUORUM_HASH)
            .expect("hex")
            .try_into()
            .expect("32 bytes");
        assert_eq!(
            *provider.fetched.lock().expect("fetched lock"),
            Some((106, signing_quorum, core_chain_locked_height)),
            "the fetch asks for exactly the quorum the proof names"
        );
        assert_eq!(
            provider.lookups.load(Ordering::SeqCst),
            2,
            "the same response is verified twice: before and after the fetch"
        );
        let mut asked = service.join().expect("quorum service");
        asked.sort();
        assert_eq!(asked, ["/previous", "/quorums"]);
    }

    /// The second verification is a full one: a key that arrived does not
    /// make a forged signature acceptable.
    #[tokio::test]
    async fn should_still_check_the_signature_after_fetching_the_key() {
        let (base_url, service) = quorum_service(vec![
            (
                "/quorums",
                200,
                current_list(SIGNING_QUORUM_HASH, &signing_quorum_key()),
            ),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();
        let response = with_proof(response, |proof| proof.signature[10] ^= 0x01);

        let error = verify(&sdk, request, response)
            .await
            .expect_err("a forged signature never verifies");

        assert!(
            matches!(
                error,
                Error::Proof(
                    drive_proof_verifier::Error::InvalidSignature { .. }
                        | drive_proof_verifier::Error::SignatureVerificationError { .. }
                        | drive_proof_verifier::Error::InvalidSignatureFormat { .. }
                )
            ),
            "got {error:?}"
        );
        assert!(
            error.can_retry(),
            "a forged signature remains attributable to the node"
        );
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        service.join().expect("quorum service");
    }

    /// A node that names a quorum the trusted service, asked after the
    /// response arrived, does not list is answerable for it: the request
    /// fails over to another node and this one is banned, as before. The
    /// response is verified only once.
    #[tokio::test]
    async fn should_report_a_quorum_the_trusted_service_does_not_list_against_the_node() {
        let (base_url, service) = quorum_service(vec![
            (
                "/quorums",
                200,
                current_list(SIGNING_QUORUM_HASH, &signing_quorum_key()),
            ),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();
        let response = with_proof(response, |proof| proof.quorum_hash = vec![0x99; 32]);

        let error = verify(&sdk, request, response)
            .await
            .expect_err("an unknown quorum never verifies");

        assert!(
            matches!(
                &error,
                Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable {
                    quorum_hash,
                    ..
                }) if *quorum_hash == [0x99; 32]
            ),
            "got {error:?}"
        );
        assert!(error.can_retry(), "a retryable proof error bans the node");
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        let mut asked = service.join().expect("quorum service");
        asked.sort();
        assert_eq!(
            asked,
            ["/previous", "/quorums"],
            "the node is held to account only after the trusted service was asked"
        );
    }

    /// When the trusted service cannot answer, the client cannot tell
    /// whether the node lied, so the node is not banned: the error is not
    /// retryable, and the retry loop only steps over the node briefly.
    #[tokio::test]
    async fn should_not_hold_an_unreachable_quorum_service_against_the_node() {
        let (base_url, service) = quorum_service(vec![
            ("/quorums", 500, "{}".to_string()),
            ("/previous", 500, "{}".to_string()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response)
            .await
            .expect_err("no key, no verification");

        assert!(
            matches!(
                error,
                Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
            ),
            "got {error:?}"
        );
        assert!(!error.can_retry(), "a non-retryable error bans no node");
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        service.join().expect("quorum service");
    }

    /// A provider that cannot fetch keys keeps today's behaviour: the
    /// verification error is reported unchanged.
    #[tokio::test]
    async fn should_report_the_original_error_when_the_provider_cannot_fetch() {
        let cannot_fetch = Counting::new(CannotFetch);
        let sdk = network_sdk(Arc::clone(&cannot_fetch));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response)
            .await
            .expect_err("the provider has no key");

        assert!(
            matches!(
                error,
                Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable { .. })
            ),
            "got {error:?}"
        );
        assert_eq!(cannot_fetch.fetches.load(Ordering::SeqCst), 1);
        assert_eq!(cannot_fetch.lookups.load(Ordering::SeqCst), 1);
        assert!(
            error.can_retry(),
            "a missing quorum remains node-attributed"
        );
    }

    /// Source outages must retain their attribution even when the provider
    /// uses only the synchronous hook, so honest responses incur no health ban.
    #[tokio::test]
    async fn should_not_ban_a_node_when_a_synchronous_quorum_source_is_unavailable() {
        let provider = Counting::new(SynchronousSourceUnavailable);
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response).await.unwrap_err();

        assert!(
            matches!(
                error,
                Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
            ),
            "a synchronous source outage must retain its attribution: {error:?}"
        );
        assert!(!error.can_retry());
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 1);
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
    }

    /// Fresh trusted absence supersedes an earlier cache error: the node
    /// named an unknown quorum and remains answerable for that response.
    #[tokio::test]
    async fn should_blame_the_node_for_authoritative_absence_after_a_cached_source_failure() {
        let (base_url, service) = quorum_service(vec![
            ("/quorums", 200, current_list(SIGNING_QUORUM_HASH, "zz")),
            ("/previous", 200, empty_previous_list()),
            ("/quorums", 200, r#"{"success":true,"data":[]}"#.to_string()),
            ("/previous", 200, empty_previous_list()),
        ]);
        let trusted = trusted_provider(base_url);
        trusted
            .refresh_quorum_caches()
            .await
            .expect("initial source lists arrive");
        tokio::time::sleep(Duration::from_secs(1)).await;
        let provider = Counting::new(trusted);
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response).await.unwrap_err();

        assert!(
            matches!(
                error,
                Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable { .. })
            ),
            "fresh authoritative absence must blame the node: {error:?}"
        );
        assert!(error.can_retry());
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        assert_eq!(service.join().expect("quorum service").len(), 4);
    }

    struct SynchronousSourceUnavailable;

    #[test]
    fn should_keep_quorum_context_when_normalizing_a_source_failure() {
        let error = Error::from(drive_proof_verifier::Error::QuorumKeyUnavailable {
            quorum_type: 106,
            quorum_hash: [0x11; 32],
            core_chain_locked_height: 2000,
            error: ContextProviderError::QuorumSourceUnavailable("offline".to_string()),
        });
        let Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(reason)) =
            error
        else {
            panic!("source error must retain its category");
        };
        assert!(reason.contains("106"), "missing quorum type: {reason}");
        assert!(
            reason.contains(&hex::encode([0x11; 32])),
            "missing quorum hash: {reason}"
        );
        assert!(reason.contains("2000"), "missing Core height: {reason}");
    }

    impl ContextProvider for SynchronousSourceUnavailable {
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
            Err(ContextProviderError::QuorumSourceUnavailable(
                "synchronous quorum source is offline".to_string(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    /// A provider with no quorum keys and no way to fetch them.
    struct CannotFetch;

    impl ContextProvider for CannotFetch {
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
            Err(ContextProviderError::InvalidQuorum(
                "not cached".to_string(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    /// Verification errors that have nothing to do with a missing key never
    /// make the provider fetch.
    #[tokio::test]
    async fn should_not_fetch_for_errors_other_than_a_missing_key() {
        let cannot_fetch = Counting::new(CannotFetch);
        let sdk = network_sdk(Arc::clone(&cannot_fetch));
        let (request, response) = recorded_epoch_fetch();
        let response = with_proof(response, |proof| proof.grovedb_proof = vec![0xff; 8]);

        let error = verify(&sdk, request, response)
            .await
            .expect_err("a corrupt proof never verifies");

        assert!(
            !matches!(
                error,
                Error::Proof(drive_proof_verifier::Error::QuorumKeyUnavailable { .. })
            ),
            "got {error:?}"
        );
        assert_eq!(cannot_fetch.fetches.load(Ordering::SeqCst), 0);
    }

    /// A key the trusted service sends that cannot be parsed is the service's
    /// fault, not the node's: no ban.
    #[test_case::test_case("zz"; "invalid_hex")]
    #[test_case::test_case(&"00".repeat(47); "wrong_length")]
    #[test_case::test_case(&"00".repeat(48); "invalid_bls_point")]
    #[test_case::test_case(&format!("c0{}", "00".repeat(47)); "identity_bls_point")]
    #[tokio::test]
    async fn should_not_hold_a_malformed_key_from_the_quorum_service_against_the_node(key: &str) {
        let (base_url, service) = quorum_service(vec![
            ("/quorums", 200, current_list(SIGNING_QUORUM_HASH, key)),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();

        for _ in 0..2 {
            let error = verify(&sdk, request, response.clone())
                .await
                .expect_err("a malformed key verifies nothing");

            assert!(
                matches!(
                    error,
                    Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
                ),
                "got {error:?}"
            );
            assert!(!error.can_retry());
        }
        service.join().expect("quorum service");
    }

    /// A provider that reports a key but whose lookup still misses it broke
    /// its contract; the node that sent the proof is not banned for that.
    #[tokio::test]
    async fn should_not_hold_a_provider_that_forgets_a_fetched_key_against_the_node() {
        let forgetful = Counting::new(Forgetful::default());
        let sdk = network_sdk(Arc::clone(&forgetful));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response)
            .await
            .expect_err("the lookup still misses");

        assert!(
            matches!(
                error,
                Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
            ),
            "got {error:?}"
        );
        assert!(!error.can_retry());
        assert_eq!(forgetful.lookups.load(Ordering::SeqCst), 2);
    }

    /// A provider that reports a key without a usable matching lookup, or
    /// fails its trusted source fetch.
    #[derive(Default)]
    struct Forgetful {
        fetch_error: bool,
        invalid_key: bool,
        fetched: AtomicBool,
    }

    impl ContextProvider for Forgetful {
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
            if self.invalid_key && self.fetched.load(Ordering::SeqCst) {
                return Ok([0; 48]);
            }
            Err(ContextProviderError::InvalidQuorum(
                "not cached".to_string(),
            ))
        }

        fn fetch_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Option<QuorumKeyFuture> {
            if self.fetch_error {
                Some(Box::pin(async {
                    Err(ContextProviderError::Generic("source offline".to_string()))
                }))
            } else {
                self.fetched.store(true, Ordering::SeqCst);
                Some(Box::pin(async { Ok(Some([7u8; 48])) }))
            }
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    #[tokio::test]
    async fn should_display_provider_fetch_failures_without_debug_variant_formatting() {
        let provider = Counting::new(Forgetful {
            fetch_error: true,
            ..Default::default()
        });
        let sdk = network_sdk(provider);
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response).await.unwrap_err();

        assert!(
            matches!(error, Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(ref reason))
                if reason == &ContextProviderError::Generic("source offline".to_string()).to_string()),
            "got {error:?}"
        );
        assert!(!error.can_retry());
    }

    #[tokio::test]
    async fn should_not_blame_a_node_for_an_invalid_key_on_the_second_verification() {
        let provider = Counting::new(Forgetful {
            invalid_key: true,
            ..Default::default()
        });
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();

        let error = verify(&sdk, request, response)
            .await
            .expect_err("the fetched key is unusable");

        assert!(
            matches!(
                error,
                Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(_))
            ),
            "got {error:?}"
        );
        assert!(
            !error.can_retry(),
            "an unusable provider key must not ban the responding node"
        );
        assert_eq!(provider.lookups.load(Ordering::SeqCst), 2);
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
    }

    fn recorded_height(response: &GetEpochsInfoResponse) -> u64 {
        match response.version.as_ref() {
            Some(EpochsVersion::V0(v0)) => v0.metadata.as_ref().expect("recorded metadata").height,
            None => panic!("recorded response is v0"),
        }
    }

    /// Other requests keep raising the SDK's height mark while one waits for
    /// a quorum key. The response that waited was fresh when it arrived, so it
    /// must be judged as of then; otherwise an honest node is banned for the
    /// client's own wait.
    #[tokio::test]
    async fn should_judge_a_response_that_waited_for_a_key_as_fresh_as_when_it_arrived() {
        let (base_url, service) = quorum_service(vec![
            (
                "/quorums",
                200,
                current_list(SIGNING_QUORUM_HASH, &signing_quorum_key()),
            ),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();
        let height = recorded_height(&response);
        *provider.raise_on_fetch.lock().expect("raise lock") =
            Some((Arc::clone(&sdk.metadata_last_seen_height), height + 10));

        let epoch = verify(&sdk, request, response)
            .await
            .expect("the response was fresh when it arrived");

        assert!(epoch.is_some());
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        service.join().expect("quorum service");
    }

    /// Waiting for a key never makes a response that was already stale when
    /// it arrived acceptable.
    #[tokio::test]
    async fn should_still_reject_a_response_that_was_stale_when_it_arrived() {
        let (base_url, service) = quorum_service(vec![
            (
                "/quorums",
                200,
                current_list(SIGNING_QUORUM_HASH, &signing_quorum_key()),
            ),
            ("/previous", 200, empty_previous_list()),
        ]);
        let provider = Counting::new(trusted_provider(base_url));
        let sdk = network_sdk(Arc::clone(&provider));
        let (request, response) = recorded_epoch_fetch();
        sdk.metadata_last_seen_height
            .fetch_max(recorded_height(&response) + 10, Ordering::SeqCst);

        let error = verify(&sdk, request, response)
            .await
            .expect_err("the response was stale when it arrived");

        assert!(matches!(error, Error::StaleNode(_)), "got {error:?}");
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
        service.join().expect("quorum service");
    }
}
