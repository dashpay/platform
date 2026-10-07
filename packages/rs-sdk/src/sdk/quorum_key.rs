//! Verifying a proof signed by a quorum the context provider has not cached.

use super::Sdk;
use crate::Error;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dash_context_provider::{ContextProvider, ContextProviderError};
use drive_proof_verifier::FromProof;

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
    /// * no answer: [`ContextProviderError::QuorumSourceUnavailable`], which is
    ///   not retryable, so no node is banned for the source's failure;
    /// * nothing, because the provider cannot fetch: the original error.
    pub(crate) async fn verify_fetching_quorum_key<R, O: FromProof<R>>(
        &self,
        request: O::Request,
        response: O::Response,
        provider: &dyn ContextProvider,
    ) -> Result<(Option<O>, ResponseMetadata, Proof), Error>
    where
        O::Request: Clone,
        O::Response: Clone,
    {
        // Both verifications use the same protocol version.
        let version = self.version();
        let missing = match O::maybe_from_proof_with_metadata(
            request.clone(),
            response.clone(),
            self.network,
            version,
            provider,
        ) {
            Ok(verified) => return Ok(verified),
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

        match fetch.await {
            Ok(Some(_)) => O::maybe_from_proof_with_metadata(
                request,
                response,
                self.network,
                version,
                provider,
            )
            .map_err(Error::from),
            Ok(None) => {
                tracing::warn!(
                    quorum_type,
                    quorum_hash = %hex::encode(quorum_hash),
                    core_chain_locked_height,
                    "proof names a quorum the trusted quorum source does not list"
                );
                Err(missing.into())
            }
            Err(ContextProviderError::QuorumSourceUnavailable(reason)) => Err(
                Error::ContextProviderError(ContextProviderError::QuorumSourceUnavailable(reason)),
            ),
            Err(error) => Err(Error::ContextProviderError(
                ContextProviderError::QuorumSourceUnavailable(error.to_string()),
            )),
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

    /// A context provider that counts the lookups and fetches the SDK makes.
    struct Counting {
        inner: Box<dyn ContextProvider>,
        lookups: AtomicUsize,
        fetches: AtomicUsize,
    }

    impl Counting {
        fn new(inner: impl ContextProvider + 'static) -> Arc<Self> {
            Arc::new(Self {
                inner: Box::new(inner),
                lookups: AtomicUsize::new(0),
                fetches: AtomicUsize::new(0),
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

        let epoch = verify(&sdk, request, response)
            .await
            .expect("the proof verifies once the signing quorum's key is fetched");

        assert!(epoch.is_some(), "the recorded epoch is proved to exist");
        assert_eq!(provider.fetches.load(Ordering::SeqCst), 1);
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
    /// retryable, and the retry loop moves on without a ban.
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

        verify(&sdk, request, response)
            .await
            .expect_err("a corrupt proof never verifies");

        assert_eq!(cannot_fetch.fetches.load(Ordering::SeqCst), 0);
    }
}
