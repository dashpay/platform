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
                Err(missing.into())
            }
            Err(ContextProviderError::QuorumSourceUnavailable(reason)) => unavailable(reason),
            Err(error) => unavailable(format!("{error:?}")),
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use crate::platform::ContextProvider;
    use crate::{Error, Sdk, SdkBuilder};
    use dapi_grpc::platform::v0::get_epochs_info_response::{
        get_epochs_info_response_v0::Result as EpochsResult, Version as EpochsVersion,
    };
    use dapi_grpc::platform::v0::{GetEpochsInfoRequest, GetEpochsInfoResponse};
    use dash_context_provider::{ContextProviderError, QuorumKeyFuture};
    use dpp::block::extended_epoch_info::ExtendedEpochInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::version::PlatformVersion;
    use rs_dapi_client::{AddressList, CanRetry, DumpData};
    use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
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
            let error = verify(&sdk, request.clone(), response.clone())
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
        let forgetful = Counting::new(Forgetful);
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

    /// A provider whose fetch reports a key that its lookup never returns.
    struct Forgetful;

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
            Some(Box::pin(async { Ok(Some([7u8; 48])) }))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
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
