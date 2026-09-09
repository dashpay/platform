// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The embedder's handle on `dash-sdk`: a tokio runtime this crate owns, an
//! `Sdk` built against the evonode endpoints the embedder discovered, and
//! the freshness policy applied to every verified response on top of the
//! SDK's own (signed time window plus monotonic height).
//!
//! Endpoints come from the embedder's deterministic masternode list, quorum
//! keys from its LLMQ store and signing from its wallet; the SDK supplies
//! query construction, transport (TLS to the evonodes), retries with
//! address banning, proof verification and protocol-version tracking.

use std::future::Future;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use dash_sdk::platform::proto::ResponseMetadata;
use dash_sdk::sdk::AddressList;
use dash_sdk::{RequestSettings, Sdk, SdkBuilder};
use platform_version::version::PlatformVersion;

use crate::provider::{Context, LocalContextProvider};
use crate::types::Meta;

/// Per-call deadline; the SDK retries across endpoints inside it.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Time budget to open a TLS connection to one evonode.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Attempts per logical request before it fails; each attempt may hit a
/// different endpoint as the SDK bans failing ones.
const RETRIES: usize = 3;
/// Largest gRPC response the SDK will decode. DAPI's servers cap messages
/// well below this; a larger blob is not a Platform response.
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
/// A verified response whose signed time is further than this from the
/// local clock comes from a stale (or replayed) block.
const TIME_TOLERANCE: Duration = Duration::from_secs(10 * 60);
/// Platform blocks a response may trail the highest verified height before
/// it is treated as stale.
const HEIGHT_TOLERANCE: u64 = 3;
/// Coarse staleness bound (in core blocks) between a proof's signed
/// core-chain-locked height and the embedder's own best ChainLock. Roughly
/// half a day at 2.5 min/block: generous so normal Platform lag never trips
/// it, small enough that a replay is bounded.
pub const MAX_CORE_CHAINLOCK_LAG: u64 = 288;
/// Worker stack size for the runtime threads that run proof verification;
/// GroveDB replay recurses deeper than a default thread stack allows.
const WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

struct Inner {
    runtime: tokio::runtime::Runtime,
    sdk: Sdk,
    endpoints: Vec<String>,
}

/// Freshness state the embedder feeds and every verified response is
/// checked against.
#[derive(Default)]
struct Freshness {
    /// Best locally verified core ChainLock height; 0 = unknown.
    local_core_chainlock_height: u32,
    /// Highest verified Platform height seen; carried across SDK rebuilds
    /// so a new endpoint set cannot serve an older state than already seen.
    last_seen_height: u64,
}

/// One SDK instance plus the context it runs against.
pub struct Client {
    provider: Arc<LocalContextProvider>,
    inner: Mutex<Option<Inner>>,
    freshness: Mutex<Freshness>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Client {
            provider: Arc::new(LocalContextProvider::default()),
            inner: Mutex::new(None),
            freshness: Mutex::new(Freshness::default()),
        }
    }

    pub fn provider(&self) -> &Arc<LocalContextProvider> {
        &self.provider
    }

    /// Installs the network context. An SDK built for a previous context is
    /// discarded; the next `set_endpoints` builds one for the new context.
    pub fn set_context(&self, context: Context) -> Result<(), String> {
        self.provider.set_context(context)?;
        // The watermarks belonged to the previous network.
        *lock(&self.freshness) = Freshness::default();
        self.shutdown();
        Ok(())
    }

    /// Replaces the evonode endpoint set (`https://host:port` URIs) and
    /// rebuilds the SDK against it. The SDK's address list is fixed at
    /// construction, so a changed masternode list means a new instance; the
    /// verified-height watermark survives the rebuild.
    pub fn set_endpoints(&self, endpoints: Vec<String>) -> Result<(), String> {
        let mut inner = lock(&self.inner);
        if let Some(current) = inner.as_ref() {
            if current.endpoints == endpoints {
                return Ok(());
            }
        }
        if endpoints.is_empty() {
            return Err("no evonode endpoints".to_string());
        }
        let context = self.provider.context()?;
        let addresses = AddressList::from_str(&endpoints.join(","))
            .map_err(|e| format!("bad evonode endpoint: {e}"))?;
        let initial_version = PlatformVersion::get(context.protocol_version)
            .map_err(|e| format!("protocol version {}: {e}", context.protocol_version))?;
        let sdk = SdkBuilder::new(addresses)
            .with_network(context.network)
            .with_proofs(true)
            .with_context_provider(Arc::clone(&self.provider))
            .with_initial_version(initial_version)
            .with_settings(RequestSettings {
                connect_timeout: Some(CONNECT_TIMEOUT),
                timeout: Some(REQUEST_TIMEOUT),
                retries: Some(RETRIES),
                ban_failed_address: Some(true),
                max_decoding_message_size: Some(MAX_RESPONSE_BYTES),
            })
            .with_time_tolerance(Some(TIME_TOLERANCE.as_millis() as u64))
            .with_height_tolerance(Some(HEIGHT_TOLERANCE))
            .with_trusted_initial_height(self.last_seen_height())
            .build()
            .map_err(|e| format!("unable to build the Platform SDK: {e}"))?;
        let previous = inner.replace(Inner {
            runtime: build_runtime()?,
            sdk,
            endpoints,
        });
        drop(inner);
        if let Some(previous) = previous {
            stop(previous);
        }
        Ok(())
    }

    /// Installs a ready-made SDK (tests use a mock one). Replaces any
    /// previous instance. Only available with the `mocks` feature: a mock
    /// SDK can answer from expectations without proof verification.
    #[cfg(feature = "mocks")]
    pub fn set_sdk(&self, sdk: Sdk) -> Result<(), String> {
        let previous = lock(&self.inner).replace(Inner {
            runtime: build_runtime()?,
            sdk,
            endpoints: Vec::new(),
        });
        if let Some(previous) = previous {
            stop(previous);
        }
        Ok(())
    }

    /// Highest Platform height verified so far (0 = none yet). Seeds the
    /// SDK's monotonic height check when the SDK is rebuilt.
    pub fn last_seen_height(&self) -> u64 {
        lock(&self.freshness).last_seen_height
    }

    /// Updates the embedder's best ChainLock height, the anchor of the
    /// core-height staleness floor. Monotonic.
    pub fn set_core_chain_locked_height(&self, height: u32) {
        let mut freshness = lock(&self.freshness);
        if height > freshness.local_core_chainlock_height {
            freshness.local_core_chainlock_height = height;
        }
    }

    /// The protocol version verified responses have shown the network to be
    /// running (at least the context's floor).
    pub fn platform_version(&self) -> Result<&'static PlatformVersion, String> {
        if let Some(inner) = lock(&self.inner).as_ref() {
            return Ok(inner.sdk.version());
        }
        let protocol_version = self.provider.context()?.protocol_version;
        PlatformVersion::get(protocol_version)
            .map_err(|e| format!("protocol version {protocol_version}: {e}"))
    }

    /// Runs one SDK operation to completion on the runtime's worker threads
    /// (large stacks: proof replay recurses), blocking the calling thread.
    /// The SDK is cloned out of the lock so concurrent callers do not
    /// serialize on it; the clone shares the address list and version state.
    pub fn run<F, Fut, T>(&self, op: F) -> Result<T, String>
    where
        F: FnOnce(Sdk) -> Fut,
        Fut: Future<Output = Result<T, dash_sdk::Error>> + Send + 'static,
        T: Send + 'static,
    {
        let (handle, sdk) = {
            let inner = lock(&self.inner);
            let inner = inner
                .as_ref()
                .ok_or("platform client has no endpoints (set_endpoints)")?;
            (inner.runtime.handle().clone(), inner.sdk.clone())
        };
        let task = handle.spawn(op(sdk));
        match handle.block_on(task) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(e.to_string()),
            Err(join) if join.is_cancelled() => Err("platform request interrupted".to_string()),
            Err(join) => Err(format!("platform request panicked: {join}")),
        }
    }

    /// Runs one metadata-returning SDK operation and applies the freshness
    /// policy to its authenticated metadata. Every proved query goes through
    /// here, so the chain-id check and the ChainLock floor cannot be skipped;
    /// [`Self::run`] stays public for the raw broadcast path and Rust callers.
    pub fn fetch<F, Fut, T>(&self, op: F) -> Result<(T, Meta), String>
    where
        F: FnOnce(Sdk) -> Fut,
        Fut: Future<Output = Result<(T, ResponseMetadata), dash_sdk::Error>> + Send + 'static,
        T: Send + 'static,
    {
        let (value, metadata) = self.run(op)?;
        Ok((value, self.accept(&metadata)?))
    }

    /// Post-verification checks on the signature-authenticated metadata of a
    /// verified response: the Tenderdash chain id must be this network's
    /// (the quorum signature covers it, so a cross-chain replay cannot forge
    /// it), and the signed core-chain-locked height must not trail the
    /// embedder's own ChainLock by more than [`MAX_CORE_CHAINLOCK_LAG`]. The
    /// SDK has already applied its signed-time window and monotonic Platform
    /// height check before the response reaches here. Until the embedder has
    /// pushed a ChainLock height the floor is inactive and the signed-time
    /// window is the only replay bound; push the height before querying.
    pub fn accept(&self, metadata: &ResponseMetadata) -> Result<Meta, String> {
        let context = self.provider.context()?;
        if metadata.chain_id != context.tenderdash_chain_id {
            return Err(format!(
                "response signed for tenderdash chain {:?}, expected {:?}",
                metadata.chain_id, context.tenderdash_chain_id
            ));
        }
        let mut freshness = lock(&self.freshness);
        let local = u64::from(freshness.local_core_chainlock_height);
        let signed = u64::from(metadata.core_chain_locked_height);
        if local > 0 && signed + MAX_CORE_CHAINLOCK_LAG < local {
            return Err(format!(
                "stale platform proof: signed core chainlock height {signed} trails the local \
                 ChainLock height {local} by more than {MAX_CORE_CHAINLOCK_LAG} blocks"
            ));
        }
        if metadata.height > freshness.last_seen_height {
            freshness.last_seen_height = metadata.height;
        }
        Ok(Meta {
            height: metadata.height,
            core_chain_locked_height: metadata.core_chain_locked_height,
            time_ms: metadata.time_ms,
            protocol_version: metadata.protocol_version,
            chain_id: metadata.chain_id.clone(),
        })
    }

    /// Cancels in-flight requests and releases the runtime. Callers blocked
    /// in [`Self::run`] return with an interruption error. Idempotent.
    pub fn shutdown(&self) {
        // Take the instance out from under the lock first: an `if let` on the
        // guard would hold it through the blocking runtime teardown and stall
        // every concurrent `run`.
        let inner = lock(&self.inner).take();
        if let Some(inner) = inner {
            stop(inner);
        }
    }
}

fn build_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("dash-platform-sdk")
        .thread_stack_size(WORKER_STACK_SIZE)
        .enable_all()
        .build()
        .map_err(|e| format!("unable to start the Platform SDK runtime: {e}"))
}

fn stop(inner: Inner) {
    inner.sdk.shutdown();
    // The connection pool is dropped inside the runtime context (tonic's
    // channels expect a reactor on drop) and before the runtime itself.
    {
        let _entered = inner.runtime.enter();
        drop(inner.sdk);
    }
    inner.runtime.shutdown_timeout(Duration::from_secs(2));
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
