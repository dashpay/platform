//! Classifying the outcome of a state-transition broadcast / result wait.
//!
//! Shared by every Platform money-moving path that must keep "never left the
//! wallet, safe to retry" apart from "may have reached a node, may still
//! execute, do not retry". Shielded spends that consume notes, shields from
//! Platform addresses and masternode credit withdrawals broadcast through
//! [`broadcast_tracking_delivery`], which keeps every dispatch's outcome so an
//! earlier dispatch that may have been delivered is never hidden by a later
//! refusal: only a refusal before anything was sent reads as failed. The
//! shield from an identity keeps the SDK's own broadcast and reads its final
//! error with [`broadcast_was_refused`], which is no proof and only decides
//! whether it waits for a result: it reports unconfirmed either way. The
//! shapes are SDK-transport facts, not operation-specific, so they live here
//! rather than behind the `shielded` feature.

/// Whether an SDK error carries Platform's own consensus verdict on the
/// transition. Two shapes qualify:
///
/// - `Error::Protocol(ProtocolError::ConsensusError(_))` — DAPI attached the
///   serialized consensus error as gRPC metadata
///   (`dash-serialized-consensus-error-bin`), which the dapi-client decodes
///   on any failed request. This is how a CheckTx rejection of the
///   transition surfaces from `broadcast()` (rs-dapi's
///   `map_broadcast_error` decodes the consensus error from Tenderdash's
///   `info` field and `TenderdashStatus` re-attaches it as metadata);
/// - a `StateTransitionBroadcastError` whose `cause` deserialized from
///   non-empty consensus `data` — the wait-stream error envelope for a
///   transition Platform executed and rejected on its merits.
///
/// Recurses through a `NoAvailableAddressesToRetry` envelope, mirroring
/// [`crate::error::as_address_invalid_nonce`].
///
/// Only these prove the transition was evaluated and REJECTED. Everything
/// else — transport errors, timeouts, `AlreadyExists` (which proves the
/// opposite: the transition is already in the mempool or on chain),
/// DAPI-internal failures, cause-less broadcast envelopes (the shape DAPI
/// uses for its own wait-side timeouts) — leaves the outcome unknown.
pub(crate) fn carries_consensus_rejection(err: &dash_sdk::Error) -> bool {
    match err {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(_)) => true,
        dash_sdk::Error::StateTransitionBroadcastError(e) => e.cause.is_some(),
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => carries_consensus_rejection(inner),
        _ => false,
    }
}

/// Whether the final error of a failed `broadcast()` call reads as a
/// refusal of the transition, rather than as a call that got no answer:
///
/// - a consensus verdict ([`carries_consensus_rejection`]): CheckTx
///   evaluated the transition and refused it;
/// - a gRPC response whose status code is a server-side rejection or a
///   connection-establishment failure: `Unavailable` is the common shape
///   of a connect-refused/offline attempt, and rejection codes
///   (`InvalidArgument`, `ResourceExhausted` = mempool full, …) say the
///   transition was refused admission;
/// - no usable DAPI addresses at all (nothing was ever sent).
///
/// This is NOT proof that the transition was left out of every mempool,
/// and it never authorizes a retry, or a release of what the transition
/// spends. An endpoint's answer is unauthenticated, and one that kept or
/// forwarded the signed transition can answer with any verdict. HTTP/2
/// stream resets after the request bytes left can surface `Unavailable`
/// too, and the dapi-client's cross-address retry only retains the LAST
/// transport error, so an earlier-attempt delivery can hide behind a
/// later attempt's final error, whatever its code. A transition reported
/// failed while it may still execute invites a second one, so every path
/// whose retry signs a new transition classifies with
/// [`broadcast_tracking_delivery`] instead: shielded spends, shields from
/// Platform addresses (to the wallet's own notes or to a recipient's) and
/// masternode credit withdrawals. The one caller left here is the shield
/// from an identity, which holds a durable guard on the exact signed
/// transition: a refusal only makes it skip the result wait for a proven
/// read of the identity nonce, and it reports unconfirmed either way.
///
/// Everything else reads as no answer, and the caller falls through to the
/// result wait: `AlreadyExists` proves
/// the tx IS in the mempool or on chain (a lost-ACK attempt was re-sent
/// by the dapi-client retry and hit tenderdash's dedupe), and
/// timeout/cancellation/no-response shapes (`TimeoutReached`,
/// `Cancelled`, gRPC `DeadlineExceeded`/`Cancelled`, plus
/// `Internal`/`Unknown`/`Aborted`/`DataLoss`, which DAPI also uses for
/// its own tenderdash-side failures that can postdate delivery) allow
/// the request to have outlived its lost ACK.
#[cfg(feature = "shielded")]
pub(crate) fn broadcast_was_refused(e: &dash_sdk::Error) -> bool {
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::Code;

    fn status_is_verdict(t: &TransportError) -> bool {
        let TransportError::Grpc(status) = t;
        !matches!(
            status.code(),
            Code::DeadlineExceeded
                | Code::Cancelled
                | Code::Unknown
                | Code::Internal
                | Code::Aborted
                | Code::DataLoss
        )
    }

    if carries_consensus_rejection(e) {
        return true;
    }
    match e {
        dash_sdk::Error::AlreadyExists(_) => false,
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(t)) => status_is_verdict(t),
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses) => true,
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(t)) => {
            status_is_verdict(t)
        }
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => broadcast_was_refused(inner),
        _ => false,
    }
}

/// What the dispatches of one transition's broadcast proved about its
/// delivery ([`broadcast_tracking_delivery`]).
#[derive(Debug)]
pub(crate) enum BroadcastDelivery {
    /// A node admitted the transition, or already held it (`AlreadyExists`).
    Admitted,
    /// No dispatch proved the transition was left out of every mempool: a
    /// dispatch reached an endpoint, which may have kept or forwarded it,
    /// whatever it answered (a consensus verdict included, which no proof
    /// or quorum signature backs), or failed in a way that allows delivery.
    /// It may execute, so the caller waits for its result, and an
    /// unconfirmed outcome keeps what the transition spends held (a shielded
    /// spend's notes stay reserved).
    MaybeDelivered(dash_sdk::Error),
    /// The transition never left the wallet: it was refused before its first
    /// dispatch was sent, there being no address to send it to or the SDK
    /// refusing it, with no dispatch sent before. Nothing holds its bytes but
    /// the wallet, so it can never execute.
    NeverDelivered(dash_sdk::Error),
}

/// How many single dispatches [`broadcast_tracking_delivery`] makes at most
/// on `sdk`: its configured overall budget, one dispatch and its effective
/// retries, so a client configured with no retries dispatches once.
pub(crate) fn spend_broadcast_dispatches(sdk: &dash_sdk::Sdk) -> usize {
    sdk.query_settings()
        .request_settings
        .finalize()
        .retries
        .saturating_add(1)
}

/// Broadcast `state_transition` (a shielded spend, a shield from Platform
/// addresses, a masternode credit withdrawal) in single dispatches (no
/// SDK retry inside a dispatch), at most the SDK's configured budget
/// ([`spend_broadcast_dispatches`]), keeping what every dispatch proved: the
/// SDK's own retry loop returns only its last error, which cannot show that
/// an earlier attempt was delivered with its response lost. Centralizing that
/// evidence in the SDK is dashpay/platform#5556; what it means for a payment's
/// notes, journal and contact stays with its caller.
///
/// Any dispatch that reached an endpoint, or failed in a way that allows it
/// (every transport error, whatever its gRPC code, a timeout, a cause-less
/// broadcast envelope, an unknown error), makes the whole broadcast
/// [`BroadcastDelivery::MaybeDelivered`]. So does a consensus verdict: an
/// endpoint's answer is unauthenticated, and one that kept or forwarded the
/// signed transition can answer with any verdict. Only a refusal before
/// anything was sent is [`BroadcastDelivery::NeverDelivered`].
pub(crate) async fn broadcast_tracking_delivery(
    sdk: &dash_sdk::Sdk,
    state_transition: &dpp::state_transition::StateTransition,
) -> BroadcastDelivery {
    use dash_sdk::dapi_client::RequestSettings;
    use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
    use dash_sdk::platform::transition::put_settings::PutSettings;

    let single_dispatch = PutSettings {
        request_settings: RequestSettings {
            retries: Some(0),
            ..RequestSettings::default()
        },
        ..PutSettings::default()
    };
    track_delivery(spend_broadcast_dispatches(sdk), || {
        state_transition.broadcast(sdk, Some(single_dispatch))
    })
    .await
}

/// The classification [`broadcast_tracking_delivery`] makes, over the
/// outcomes `dispatch` returns, one call per single dispatch, at most
/// `dispatches` of them.
async fn track_delivery<F, Fut>(dispatches: usize, mut dispatch: F) -> BroadcastDelivery
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), dash_sdk::Error>>,
{
    use dash_sdk::dapi_client::DapiClientError;

    let mut maybe_delivered: Option<dash_sdk::Error> = None;
    for _ in 0..dispatches.max(1) {
        let error = match dispatch().await {
            Ok(()) | Err(dash_sdk::Error::AlreadyExists(_)) => return BroadcastDelivery::Admitted,
            Err(error) => error,
        };
        // Refused before anything was sent: no address, or the SDK's own
        // refusal of a kind the network does not serve yet.
        let never_sent = matches!(
            error,
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
                | dash_sdk::Error::ProtocolVersionNotActive { .. }
        ) || matches!(
            &error,
            dash_sdk::Error::NoAvailableAddressesToRetry(inner)
                if matches!(
                    **inner,
                    dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
                )
        );
        if never_sent {
            return match maybe_delivered {
                Some(_) => BroadcastDelivery::MaybeDelivered(error),
                None => BroadcastDelivery::NeverDelivered(error),
            };
        }
        // An endpoint's verdict ends the dispatches, and proves nothing.
        if carries_consensus_rejection(&error) {
            return BroadcastDelivery::MaybeDelivered(error);
        }
        maybe_delivered = Some(error);
    }
    match maybe_delivered {
        Some(error) => BroadcastDelivery::MaybeDelivered(error),
        // Unreachable: every pass sets it or returns.
        None => BroadcastDelivery::MaybeDelivered(dash_sdk::Error::Generic(
            "no broadcast dispatch was made".to_string(),
        )),
    }
}

/// Network SDKs for the tests of the flows that broadcast through
/// [`broadcast_tracking_delivery`].
#[cfg(test)]
pub(crate) mod test_sdks {
    use dash_sdk::dpp::dashcore::Network;
    use dash_sdk::dpp::prelude::CoreBlockHeight;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::platform::ContextProvider;
    use dash_sdk::sdk::AddressList;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::str::FromStr;
    use std::sync::Arc;

    /// A context provider that knows no contract and no quorum key: the
    /// tests' requests fail before a proof is verified.
    struct NoKeys;

    impl ContextProvider for NoKeys {
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
            Err(ContextProviderError::Generic("no quorum keys".to_string()))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    fn sdk(addresses: AddressList) -> dash_sdk::Sdk {
        dash_sdk::SdkBuilder::new(addresses)
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .with_context_provider(NoKeys)
            .build()
            .expect("a network SDK")
    }

    /// A network SDK with no address to send to: every dispatch is refused
    /// before anything is sent.
    pub(crate) fn sdk_without_addresses() -> dash_sdk::Sdk {
        sdk(AddressList::new())
    }

    /// A network SDK whose only node refuses the connection, on a loopback
    /// port nothing listens on: every dispatch fails in transport, the way
    /// an offline attempt does.
    pub(crate) fn sdk_refusing_connections() -> dash_sdk::Sdk {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        drop(listener);
        sdk(AddressList::from_str(&format!("http://{address}")).expect("an address"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::{Code, Status};
    use std::sync::Mutex;

    /// Another error of the same shape as `error`, which is not `Clone`.
    fn copy(error: &dash_sdk::Error) -> dash_sdk::Error {
        match error {
            dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                status,
            ))) => grpc(status.code()),
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses) => {
                dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses)
            }
            dash_sdk::Error::NoAvailableAddressesToRetry(inner) => {
                dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(copy(inner)))
            }
            dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(_)) => rejection(),
            dash_sdk::Error::ProtocolVersionNotActive {
                method,
                required,
                feature,
                network,
                pinned,
                ..
            } => dash_sdk::Error::ProtocolVersionNotActive {
                method,
                required: *required,
                feature: *feature,
                network: *network,
                pinned: *pinned,
                cause: None,
            },
            other => dash_sdk::Error::Generic(other.to_string()),
        }
    }

    fn grpc(code: Code) -> dash_sdk::Error {
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
            Status::new(code, "test"),
        )))
    }

    /// A consensus rejection, as DAPI attaches one to a refused broadcast.
    fn rejection() -> dash_sdk::Error {
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::basic::UnsupportedFeatureError;
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            dpp::consensus::ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
                UnsupportedFeatureError::new("test".to_string(), 1),
            )),
        )))
    }

    /// What [`track_delivery`] makes of dispatches whose outcome `outcome`
    /// gives by dispatch number, from 0, and how many dispatches it made.
    async fn tracked(
        outcome: impl Fn(usize) -> Result<(), dash_sdk::Error>,
    ) -> (BroadcastDelivery, usize) {
        let dispatched = Mutex::new(0usize);
        let delivery = track_delivery(BUDGET, || {
            let mut count = dispatched.lock().expect("count");
            let result = outcome(*count);
            *count += 1;
            std::future::ready(result)
        })
        .await;
        let dispatched = *dispatched.lock().expect("count");
        (delivery, dispatched)
    }

    /// The dispatch budget the tests run with.
    const BUDGET: usize = 6;

    /// An endpoint that kept or forwarded the transition can answer the
    /// first dispatch with a consensus verdict, as serialized metadata the
    /// SDK decodes with no proof behind it: the broadcast stays maybe
    /// delivered, so its payment stays unconfirmed with its notes, journal
    /// record and contact hold, and the dispatches end there.
    #[tokio::test]
    async fn should_keep_a_broadcast_an_endpoint_refused_maybe_delivered() {
        use dash_sdk::dapi_grpc::tonic::metadata::{MetadataMap, MetadataValue};
        use dpp::serialization::PlatformSerializableWithPlatformVersion;
        use dpp::version::PlatformVersion;

        let verdict = || {
            let bytes = dpp::consensus::ConsensusError::BasicError(
                dpp::consensus::basic::BasicError::UnsupportedFeatureError(
                    dpp::consensus::basic::UnsupportedFeatureError::new("test".to_string(), 1),
                ),
            )
            .serialize_to_bytes_with_platform_version(PlatformVersion::latest())
            .expect("serialized");
            let mut metadata = MetadataMap::new();
            metadata.insert_bin(
                "dash-serialized-consensus-error-bin",
                MetadataValue::from_bytes(&bytes),
            );
            dash_sdk::Error::from(DapiClientError::Transport(TransportError::Grpc(
                Status::with_metadata(Code::InvalidArgument, "rejected", metadata),
            )))
        };
        assert!(carries_consensus_rejection(&verdict()), "{:?}", verdict());
        let (delivery, dispatched) = tracked(|_| Err(verdict())).await;
        assert!(
            matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
            "{delivery:?}"
        );
        assert_eq!(dispatched, 1);
    }

    /// The tracker keeps the SDK's configured budget: a client configured
    /// with no retries dispatches once, one with three retries four times.
    #[tokio::test]
    async fn should_keep_the_sdk_s_configured_retry_budget() {
        use dash_sdk::dapi_client::RequestSettings;

        let no_retries = dash_sdk::SdkBuilder::new_mock()
            .with_settings(RequestSettings {
                retries: Some(0),
                ..RequestSettings::default()
            })
            .build()
            .expect("an SDK");
        assert_eq!(spend_broadcast_dispatches(&no_retries), 1);
        let three_retries = dash_sdk::SdkBuilder::new_mock()
            .with_settings(RequestSettings {
                retries: Some(3),
                ..RequestSettings::default()
            })
            .build()
            .expect("an SDK");
        assert_eq!(spend_broadcast_dispatches(&three_retries), 4);

        let dispatched = Mutex::new(0usize);
        let delivery = track_delivery(spend_broadcast_dispatches(&no_retries), || {
            *dispatched.lock().expect("count") += 1;
            std::future::ready(Err(grpc(Code::Unavailable)))
        })
        .await;
        assert!(matches!(delivery, BroadcastDelivery::MaybeDelivered(_)));
        assert_eq!(*dispatched.lock().expect("count"), 1);
    }

    /// An attempt admitted with its response lost, then retries refused for
    /// any reason, leaves the broadcast maybe delivered: the last error
    /// alone cannot show the transition never left.
    #[tokio::test]
    async fn should_keep_an_earlier_ambiguous_dispatch_over_later_refusals() {
        for lost in [Code::Unavailable, Code::DeadlineExceeded] {
            for refusal in [
                grpc(Code::ResourceExhausted),
                grpc(Code::Unimplemented),
                grpc(Code::InvalidArgument),
                rejection(),
            ] {
                let refusal = &refusal;
                let (delivery, _) = tracked(|dispatch| match dispatch {
                    0 => Err(grpc(lost)),
                    _ => Err(copy(refusal)),
                })
                .await;
                assert!(
                    matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
                    "{lost:?} then a refusal: {delivery:?}"
                );
            }
        }
    }

    /// Every transport-level failure may have delivered the transition,
    /// whatever its code, so a broadcast that only ever fails so is maybe
    /// delivered after the whole budget.
    #[tokio::test]
    async fn should_treat_every_transport_failure_as_maybe_delivered() {
        for code in [
            Code::Unavailable,
            Code::ResourceExhausted,
            Code::Unimplemented,
            Code::InvalidArgument,
            Code::Internal,
        ] {
            let (delivery, dispatched) = tracked(|_| Err(grpc(code))).await;
            assert!(
                matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
                "{code:?}: {delivery:?}"
            );
            assert_eq!(dispatched, BUDGET, "{code:?}");
        }
    }

    /// Only a refusal before anything was sent, nowhere to send the
    /// transition or the SDK refusing it, with no dispatch sent before,
    /// shows it never left.
    #[tokio::test]
    async fn should_count_a_broadcast_never_delivered_only_without_an_earlier_ambiguity() {
        for first in [
            dash_sdk::Error::ProtocolVersionNotActive {
                method: "broadcast_state_transition",
                required: 15,
                feature: None,
                network: 14,
                pinned: false,
                cause: None,
            },
            dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(
                dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses),
            )),
        ] {
            let first = &first;
            let (delivery, dispatched) = tracked(|_| Err(copy(first))).await;
            assert!(
                matches!(delivery, BroadcastDelivery::NeverDelivered(_)),
                "{delivery:?}"
            );
            assert_eq!(dispatched, 1);
        }
    }

    /// The broadcast of a signed transition through a network SDK: with no
    /// address to send it to it never left, and a node that refuses the
    /// connection leaves it maybe delivered, as a transport failure cannot
    /// show nothing was sent. Here an identity credit withdrawal, which is
    /// broadcast in every build.
    #[tokio::test]
    async fn should_track_the_delivery_of_a_broadcast_through_a_network_sdk() {
        use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
        use dpp::state_transition::StateTransition;

        let transition: StateTransition = IdentityCreditWithdrawalTransitionV1 {
            amount: 1_000_000,
            nonce: 1,
            ..Default::default()
        }
        .into();

        let delivery =
            broadcast_tracking_delivery(&test_sdks::sdk_without_addresses(), &transition).await;
        assert!(
            matches!(delivery, BroadcastDelivery::NeverDelivered(_)),
            "{delivery:?}"
        );
        let delivery =
            broadcast_tracking_delivery(&test_sdks::sdk_refusing_connections(), &transition).await;
        assert!(
            matches!(delivery, BroadcastDelivery::MaybeDelivered(_)),
            "{delivery:?}"
        );
    }

    /// A dispatch a node admitted, or already holds, is admitted, after any
    /// number of failed ones.
    #[tokio::test]
    async fn should_count_an_admitted_or_known_dispatch_as_admitted() {
        let (delivery, _) = tracked(|dispatch| match dispatch {
            0 => Err(grpc(Code::Unavailable)),
            _ => Ok(()),
        })
        .await;
        assert!(
            matches!(delivery, BroadcastDelivery::Admitted),
            "{delivery:?}"
        );
        let (delivery, _) = tracked(|dispatch| match dispatch {
            0 => Err(grpc(Code::DeadlineExceeded)),
            _ => Err(dash_sdk::Error::AlreadyExists("in the mempool".to_string())),
        })
        .await;
        assert!(
            matches!(delivery, BroadcastDelivery::Admitted),
            "{delivery:?}"
        );
    }
}
