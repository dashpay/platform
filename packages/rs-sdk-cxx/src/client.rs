// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The bridge's `PlatformClient`: one `Sdk` built from the embedder's
//! config, and the entry points, each a forward to one `dash-sdk` call run
//! on the binding's [`Runtime`].

use std::collections::BTreeMap;
use std::future::{ready, Future};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use dash_sdk::dapi_client::transport::{ProxyEndpoint, Socks5Auth, Socks5Proxy};
use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::document::DocumentV0Getters;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::v0::IdentityV0;
use dash_sdk::dpp::identity::{Identity, IdentityPublicKey};
use dash_sdk::dpp::key_wallet::bip32::DerivationPath;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::dpp::secp256k1::{PublicKey, SecretKey};
use dash_sdk::dpp::ProtocolError;
use dash_sdk::platform::dashpay::{
    ContactRequestInput, EcdhProvider, RecipientIdentity, SendContactRequestInput,
};
use dash_sdk::platform::dpns_usernames::{convert_to_homograph_safe_chars, RegisterDpnsNameInput};
use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;
use dash_sdk::platform::types::identity::PublicKeyHash;
use dash_sdk::platform::Fetch;
use dash_sdk::sdk::{Address, AddressList};
use dash_sdk::{Error, Sdk, SdkBuilder};

use crate::context::CoreContextProvider;
use crate::ffi::{self, Status, StatusKind};
use crate::runtime::{blocking, lock, RunError, Runtime};
use crate::signer::WalletSigner;
use crate::{guarded, marshal};

pub struct PlatformClient {
    runtime: Runtime,
    /// Taken by `shutdown`, once no call holds a clone.
    sdk: Mutex<Option<Sdk>>,
}

impl Status {
    fn new(kind: StatusKind, message: impl Into<String>) -> Self {
        Status {
            kind,
            consensus_code: 0,
            message: message.into(),
        }
    }

    fn ok() -> Self {
        Self::new(StatusKind::Ok, "")
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusKind::Internal, message)
    }

    fn not_found(what: impl Into<String>) -> Self {
        Self::new(StatusKind::NotFound, what)
    }
}

/// Lets `?` turn a failed input conversion into `Internal` ...
impl From<String> for Status {
    fn from(message: String) -> Self {
        Status::internal(message)
    }
}

/// ... and an SDK error into its [`classify`] status.
impl From<Error> for Status {
    fn from(error: Error) -> Self {
        classify(&error)
    }
}

/// How an SDK error reads to the embedder. Only `Ok` and `NotFound` carry
/// a proof; everything else must be treated as "unknown".
pub fn classify(error: &Error) -> Status {
    let kind = match error {
        Error::NoAvailableAddressesToRetry(last) => return classify(last),
        Error::Protocol(ProtocolError::ConsensusError(consensus)) => {
            return Status {
                consensus_code: consensus.code(),
                ..Status::new(StatusKind::Consensus, consensus.to_string())
            }
        }
        Error::StateTransitionBroadcastError(broadcast) => {
            return Status {
                consensus_code: broadcast.code,
                ..Status::new(StatusKind::Consensus, broadcast.message.clone())
            }
        }
        _ if error.is_quorum_source_unavailable() => StatusKind::Unavailable,
        Error::DapiClientError(_)
        | Error::TimeoutReached(..)
        | Error::Cancelled(_)
        | Error::StaleNode(_) => StatusKind::Unavailable,
        Error::Proof(_)
        | Error::DriveProofError(..)
        | Error::InvalidProvedResponse(_)
        | Error::ExecutionNotProved(_)
        | Error::ContextProviderError(_) => StatusKind::Rejected,
        _ => StatusKind::Internal,
    };
    Status::new(kind, error.to_string())
}

/// One result struct per value type, each built from a call's outcome.
macro_rules! result_from {
    ($result:ident { $field:ident: $value:ty = $empty:expr }) => {
        impl From<Result<$value, Status>> for ffi::$result {
            fn from(outcome: Result<$value, Status>) -> Self {
                match outcome {
                    Ok($field) => ffi::$result {
                        status: Status::ok(),
                        $field,
                    },
                    Err(status) => ffi::$result {
                        status,
                        $field: $empty,
                    },
                }
            }
        }
    };
}

result_from!(IdentityResult {
    identity: ffi::Identity = Default::default()
});
result_from!(IdResult {
    id: [u8; 32] = [0; 32]
});
result_from!(BalanceResult { balance: u64 = 0 });
result_from!(VoteStateResult {
    state: ffi::VoteState = marshal::empty_vote_state()
});
result_from!(ContestsResult { contests: Vec<ffi::Contest> = Vec::new() });
result_from!(ContactRequestsResult { requests: Vec<ffi::ContactRequest> = Vec::new() });

impl From<Result<(Vec<ffi::DpnsName>, bool), Status>> for ffi::NamesResult {
    fn from(outcome: Result<(Vec<ffi::DpnsName>, bool), Status>) -> Self {
        let (status, (names, complete)) = match outcome {
            Ok(value) => (Status::ok(), value),
            Err(status) => (status, (Vec::new(), false)),
        };
        ffi::NamesResult {
            status,
            names,
            complete,
        }
    }
}

impl PlatformClient {
    pub fn new(
        config: &ffi::Config,
        context: cxx::UniquePtr<ffi::CoreContext>,
    ) -> Result<Self, String> {
        let network = match config.network {
            ffi::Network::Mainnet => Network::Mainnet,
            ffi::Network::Testnet => Network::Testnet,
            ffi::Network::Devnet => Network::Devnet,
            ffi::Network::Regtest => Network::Regtest,
            other => return Err(format!("unknown network {}", other.repr)),
        };
        let proxy = proxy(&config.proxy)?;
        let addresses = config
            .endpoints
            .iter()
            .map(|uri| endpoint(uri, proxy.is_some()))
            .collect::<Result<AddressList, String>>()?;
        if context.is_null() {
            return Err("a CoreContext is required".to_string());
        }
        let provider = CoreContextProvider {
            source: context,
            platform_llmq_type: config.platform_llmq_type,
        };
        let mut builder = SdkBuilder::new(addresses)
            .with_network(network)
            .with_context_provider(provider);
        if let Some(proxy) = proxy {
            builder = builder.with_proxy(proxy);
        }
        let runtime = Runtime::new()?;
        let sdk = {
            let _context = runtime.handle.enter();
            builder
                .build()
                .map_err(|e| format!("unable to build the SDK: {e}"))?
        };
        Ok(PlatformClient {
            runtime,
            sdk: Mutex::new(Some(sdk)),
        })
    }

    /// Runs the future `op` builds from the SDK on the runtime, blocking
    /// until it returns. Input checks belong inside it: a client that is shut
    /// down answers `Unavailable` whatever the input.
    fn call<T, Fut>(&self, what: &str, op: impl FnOnce(Sdk) -> Fut) -> Result<T, Status>
    where
        Fut: Future<Output = Result<T, Status>> + Send + 'static,
        T: Send + 'static,
    {
        guarded(
            what,
            || {
                let shut_down = || Status::new(StatusKind::Unavailable, "client is shut down");
                // Held until the SDK clone and the future are gone.
                let call = self.runtime.call().ok_or_else(shut_down)?;
                let sdk = lock(&self.sdk).clone().ok_or_else(shut_down)?;
                match call.run(op(sdk)) {
                    Ok(outcome) => outcome,
                    Err(RunError::ShutDown) => Err(shut_down()),
                    Err(RunError::Panicked(message)) => Err(message.into()),
                }
            },
            |message| Err(message.into()),
        )
    }

    /// Cancels without cloning the SDK, so the one this client holds is its
    /// last owner: dropped, with the `CoreContext`, inside the serialized
    /// runtime shutdown once every call has ended (and inside the runtime
    /// context, as tonic's pooled channels expect).
    pub fn shutdown(&self) {
        guarded(
            "shutdown",
            || {
                if let Some(sdk) = lock(&self.sdk).as_ref() {
                    sdk.shutdown();
                }
                self.runtime.shutdown(|| drop(lock(&self.sdk).take()));
            },
            |_| (),
        )
    }

    pub fn get_identity(&self, id: &[u8; 32]) -> ffi::IdentityResult {
        self.call("get_identity", |sdk| {
            let id = Identifier::from(*id);
            async move { fetched_identity(Identity::fetch(&sdk, id).await) }
        })
        .into()
    }

    pub fn get_identity_by_pubkey_hash(&self, pubkey_hash: &[u8; 20]) -> ffi::IdentityResult {
        self.call("get_identity_by_pubkey_hash", |sdk| {
            let hash = PublicKeyHash(*pubkey_hash);
            async move { fetched_identity(Identity::fetch(&sdk, hash).await) }
        })
        .into()
    }

    pub fn resolve_name(&self, name: &str) -> ffi::IdResult {
        self.call("resolve_name", |sdk| {
            let name = name.to_string();
            async move {
                sdk.resolve_dpns_name(&name)
                    .await?
                    .map(|id| id.to_buffer())
                    .ok_or_else(|| Status::not_found(format!("no DPNS name {name}")))
            }
        })
        .into()
    }

    pub fn names_of_identity(
        &self,
        identity: &[u8; 32],
        page_limit: u32,
        max_pages: u32,
    ) -> ffi::NamesResult {
        self.call("names_of_identity", |sdk| {
            let identity = Identifier::from(*identity);
            async move {
                if page_limit == 0 || max_pages == 0 {
                    return Err(Status::internal("page_limit and max_pages must be >= 1"));
                }
                let (names, complete) = sdk
                    .get_all_dpns_usernames_by_identity(identity, page_limit, max_pages as usize)
                    .await?;
                Ok((
                    names.into_iter().map(marshal::dpns_name).collect(),
                    complete,
                ))
            }
        })
        .into()
    }

    /// `complete` is always false: the SDK drops unparseable documents after
    /// the fetch and does not report the raw page length, so even a short
    /// result does not prove the end.
    pub fn search_names(&self, prefix: &str, limit: u32) -> ffi::NamesResult {
        self.call("search_names", |sdk| {
            let prefix = prefix.to_string();
            async move {
                if limit == 0 {
                    return Err(Status::internal("limit must be >= 1"));
                }
                let names = sdk.search_dpns_names(&prefix, Some(limit)).await?;
                Ok((names.into_iter().map(marshal::dpns_name).collect(), false))
            }
        })
        .into()
    }

    pub fn pending_contests_for_identity(&self, identity: &[u8; 32]) -> ffi::ContestsResult {
        self.call("pending_contests_for_identity", |sdk| {
            let identity = Identifier::from(*identity);
            async move {
                let contests = sdk
                    .get_non_resolved_dpns_contests_for_identity_strict(identity, None)
                    .await?;
                Ok(contests
                    .into_iter()
                    .map(|(name, info)| ffi::Contest {
                        name,
                        ends_at_ms: info.end_time,
                        state: marshal::vote_state(&info.contenders),
                    })
                    .collect())
            }
        })
        .into()
    }

    pub fn contested_vote_state(&self, label: &str) -> ffi::VoteStateResult {
        self.call("contested_vote_state", |sdk| {
            // The SDK's contested query takes the index value as is.
            let label = convert_to_homograph_safe_chars(label);
            async move {
                let contenders = sdk.get_contested_dpns_vote_state(&label, None).await?;
                Ok(marshal::vote_state(&contenders))
            }
        })
        .into()
    }

    pub fn contact_requests(
        &self,
        identity: &[u8; 32],
        direction: ffi::ContactDirection,
        since_ms: u64,
    ) -> ffi::ContactRequestsResult {
        self.call("contact_requests", |sdk| {
            let identity = Identifier::from(*identity);
            let since = (since_ms != 0).then_some(since_ms);
            async move {
                let documents = match direction {
                    ffi::ContactDirection::Sent => {
                        sdk.fetch_sent_contact_requests(identity, since).await
                    }
                    ffi::ContactDirection::Received => {
                        sdk.fetch_received_contact_requests(identity, since).await
                    }
                    other => {
                        return Err(Status::internal(format!(
                            "unknown direction {}",
                            other.repr
                        )))
                    }
                }?;
                documents
                    .values()
                    .flatten()
                    .map(marshal::contact_request)
                    .collect::<Result<_, _>>()
                    .map_err(|e| Status::new(StatusKind::Rejected, e))
            }
        })
        .into()
    }

    pub fn register_identity(
        &self,
        keys: &[ffi::IdentityKey],
        proof: &ffi::AssetLockProof,
        asset_lock_key_ref: u32,
        signer: cxx::UniquePtr<ffi::WalletSigner>,
    ) -> ffi::IdentityResult {
        self.call("register_identity", |sdk| {
            let public_keys = keys
                .iter()
                .map(|key| marshal::new_identity_key(key).map(|key| (key.id(), key)))
                .collect::<Result<BTreeMap<_, _>, String>>()
                .and_then(|public_keys| {
                    (public_keys.len() == keys.len())
                        .then_some(public_keys)
                        .ok_or_else(|| "duplicate identity key ids".to_string())
                });
            let proof = marshal::asset_lock_proof(proof);
            async move {
                // The id is derived from the asset lock when the transition is built.
                let identity = bare_identity(Identifier::default(), public_keys?);
                let signer = WalletSigner::new(signer)?;
                let created = identity
                    .put_to_platform_and_wait_for_response_with_signer(
                        &sdk,
                        proof?,
                        &DerivationPath::master(),
                        &signer.asset_lock(asset_lock_key_ref),
                        &signer,
                        None,
                    )
                    .await?;
                Ok(marshal::identity(&created))
            }
        })
        .into()
    }

    pub fn top_up_identity(
        &self,
        identity: &[u8; 32],
        proof: &ffi::AssetLockProof,
        asset_lock_key_ref: u32,
        signer: cxx::UniquePtr<ffi::WalletSigner>,
    ) -> ffi::BalanceResult {
        self.call("top_up_identity", |sdk| {
            // A top-up names only the identity's id.
            let identity = bare_identity(Identifier::from(*identity), BTreeMap::new());
            let proof = marshal::asset_lock_proof(proof);
            async move {
                let signer = WalletSigner::new(signer)?;
                Ok(identity
                    .top_up_identity_with_signer(
                        &sdk,
                        proof?,
                        &DerivationPath::master(),
                        &signer.asset_lock(asset_lock_key_ref),
                        None,
                    )
                    .await?)
            }
        })
        .into()
    }

    pub fn register_dpns_name(
        &self,
        label: &str,
        identity: &[u8; 32],
        key_id: u32,
        contest_fund: u64,
        signer: cxx::UniquePtr<ffi::WalletSigner>,
    ) -> ffi::IdResult {
        self.call("register_dpns_name", |sdk| {
            let label = label.to_string();
            let identity = Identifier::from(*identity);
            async move {
                let signer = WalletSigner::new(signer)?;
                let (identity, key) = identity_and_key(&sdk, identity, key_id).await?;
                let registered = sdk
                    .register_dpns_name(RegisterDpnsNameInput {
                        label,
                        identity,
                        identity_public_key: key,
                        signer,
                        preorder_callback: None,
                        contest_fund: (contest_fund != 0).then_some(contest_fund),
                    })
                    .await?;
                Ok(registered.domain_document.id().to_buffer())
            }
        })
        .into()
    }

    pub fn send_contact_request(
        &self,
        input: &ffi::ContactRequestInput,
        signer: cxx::UniquePtr<ffi::WalletSigner>,
        keys: cxx::UniquePtr<ffi::ContactKeys>,
    ) -> ffi::IdResult {
        self.call("send_contact_request", |sdk| {
            let input = input.clone();
            async move {
                let signer = WalletSigner::new(signer)?;
                if keys.is_null() {
                    return Err(Status::internal("ContactKeys are required"));
                }
                let ecdh_keys = Arc::new(keys);
                let xpub_keys = Arc::clone(&ecdh_keys);
                let sender_key = input.sender_key_index;
                let recipient = input.recipient;
                let ecdh = EcdhProvider::<NoPrivateKey, _, _, _>::ClientSide {
                    get_shared_secret: move |recipient_key: &PublicKey| {
                        let mut secret = [0u8; 32];
                        let answered = blocking(|| {
                            ecdh_keys.SharedSecret(
                                sender_key,
                                &recipient_key.serialize(),
                                &mut secret,
                            )
                        });
                        ready(answered.then_some(secret).ok_or_else(|| {
                            Error::Generic("the wallet refused the ECDH".to_string())
                        }))
                    },
                };
                let xpub = move |account_reference: u32| {
                    let mut xpub = [0u8; 69];
                    let answered = blocking(|| {
                        xpub_keys.CompactXpub(&recipient, account_reference, &mut xpub)
                    });
                    ready(answered.then(|| xpub.to_vec()).ok_or_else(|| {
                        Error::Generic("the wallet refused the contact xpub".to_string())
                    }))
                };
                let (sender, key) =
                    identity_and_key(&sdk, Identifier::from(input.sender), input.signing_key_id)
                        .await?;
                let request = ContactRequestInput {
                    sender_identity: sender,
                    recipient: RecipientIdentity::Identifier(Identifier::from(input.recipient)),
                    sender_key_index: input.sender_key_index,
                    recipient_key_index: input.recipient_key_index,
                    account_reference: input.account_reference,
                    account_label: (!input.account_label.is_empty()).then_some(input.account_label),
                    auto_accept_proof: None,
                };
                let sent = sdk
                    .send_contact_request(
                        SendContactRequestInput {
                            contact_request: request,
                            identity_public_key: key,
                            signer,
                        },
                        ecdh,
                        xpub,
                    )
                    .await?;
                Ok(sent.document.id().to_buffer())
            }
        })
        .into()
    }
}

impl Drop for PlatformClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// The SDK-side ECDH branch, never used: the wallet derives the secret.
type NoPrivateKey = fn(&IdentityPublicKey, u32) -> std::future::Ready<Result<SecretKey, Error>>;

fn fetched_identity(fetched: Result<Option<Identity>, Error>) -> Result<ffi::Identity, Status> {
    fetched?
        .map(|identity| marshal::identity(&identity))
        .ok_or_else(|| Status::not_found("no such identity"))
}

fn bare_identity(id: Identifier, public_keys: BTreeMap<u32, IdentityPublicKey>) -> Identity {
    IdentityV0 {
        id,
        public_keys,
        balance: 0,
        revision: 0,
    }
    .into()
}

/// The identity and its key `key_id`, as Platform holds them now.
async fn identity_and_key(
    sdk: &Sdk,
    id: Identifier,
    key_id: u32,
) -> Result<(Identity, IdentityPublicKey), Status> {
    let identity = Identity::fetch(sdk, id)
        .await?
        .ok_or_else(|| Status::not_found(format!("no identity {id}")))?;
    let key = identity
        .public_keys()
        .get(&key_id)
        .cloned()
        .ok_or_else(|| Status::internal(format!("identity {id} has no key {key_id}")))?;
    Ok((identity, key))
}

/// The embedder's proxy setting. Anything malformed is an error, so a
/// client never connects directly when a proxy was meant.
fn proxy(cfg: &ffi::Proxy) -> Result<Option<Socks5Proxy>, String> {
    let endpoint = match cfg.kind {
        ffi::ProxyKind::None if cfg.address.is_empty() => return Ok(None),
        ffi::ProxyKind::None => return Err("a proxy address without a proxy kind".to_string()),
        ffi::ProxyKind::Tcp => ProxyEndpoint::Tcp(
            SocketAddr::from_str(&cfg.address)
                .map_err(|_| format!("proxy {:?} is not a numeric ip:port", cfg.address))?,
        ),
        #[cfg(unix)]
        ffi::ProxyKind::Unix if !cfg.address.is_empty() => {
            ProxyEndpoint::Unix(cfg.address.clone().into())
        }
        other => {
            return Err(format!(
                "unusable proxy kind {} {:?}",
                other.repr, cfg.address
            ))
        }
    };
    let auth = if cfg.isolate {
        Socks5Auth::RandomPerConnection
    } else {
        Socks5Auth::None
    };
    Ok(Some(Socks5Proxy { endpoint, auth }))
}

/// An evonode endpoint: `https`, and a literal IP, or an onion name when a
/// proxy can reach it. Nothing is resolved locally.
fn endpoint(uri: &str, proxied: bool) -> Result<Address, String> {
    let address = Address::from_str(uri).map_err(|e| format!("bad endpoint {uri}: {e}"))?;
    if address.uri().scheme_str() != Some("https") {
        return Err(format!("bad endpoint {uri}: not https"));
    }
    let host = address.uri().host().unwrap_or_default();
    let ip = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if ip.parse::<IpAddr>().is_ok() || (proxied && host.ends_with(".onion")) {
        Ok(address)
    } else {
        Err(format!(
            "bad endpoint {uri}: the host must be an IP address, or an onion name with a proxy"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dpp::consensus::ConsensusError;
    use dash_sdk::error::{ContextProviderError, StateTransitionBroadcastError};

    /// Only a proof yields `Ok` or `NotFound`; consensus refusals keep
    /// their code; nothing else may read as either.
    #[test]
    fn sdk_errors_map_to_the_status_table() {
        let status = classify(&Error::Protocol(ProtocolError::ConsensusError(Box::new(
            ConsensusError::DefaultError,
        ))));
        assert_eq!(status.kind, StatusKind::Consensus);
        let status = classify(&Error::StateTransitionBroadcastError(
            StateTransitionBroadcastError {
                code: 40100,
                message: "refused".to_string(),
                cause: None,
            },
        ));
        assert_eq!(
            (status.kind, status.consensus_code),
            (StatusKind::Consensus, 40100)
        );

        let quorum = Error::ContextProviderError;
        for (error, kind) in [
            (
                Error::DapiClientError(DapiClientError::NoAvailableAddresses),
                StatusKind::Unavailable,
            ),
            (
                Error::TimeoutReached(std::time::Duration::from_secs(1), "t".into()),
                StatusKind::Unavailable,
            ),
            (
                quorum(ContextProviderError::QuorumSourceUnavailable("x".into())),
                StatusKind::Unavailable,
            ),
            (
                quorum(ContextProviderError::InvalidQuorum("x".into())),
                StatusKind::Rejected,
            ),
            (
                Error::NoAvailableAddressesToRetry(Box::new(Error::InvalidProvedResponse(
                    "x".into(),
                ))),
                StatusKind::Rejected,
            ),
            (Error::Generic("x".into()), StatusKind::Internal),
        ] {
            assert_eq!(classify(&error).kind, kind, "{error}");
        }
    }

    /// Tor stream isolation; the C++ smoke covers the refusals.
    #[test]
    fn isolate_asks_for_fresh_proxy_credentials() {
        let proxy = |isolate| {
            super::proxy(&ffi::Proxy {
                kind: ffi::ProxyKind::Tcp,
                address: "127.0.0.1:9050".to_string(),
                isolate,
            })
            .unwrap()
            .unwrap()
            .auth
        };
        assert_eq!(proxy(true), Socks5Auth::RandomPerConnection);
        assert_eq!(proxy(false), Socks5Auth::None);
    }
}
