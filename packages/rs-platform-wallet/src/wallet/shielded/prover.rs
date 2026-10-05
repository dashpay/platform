//! Cached Orchard prover for zero-knowledge proof generation.
//!
//! The Halo 2 proving key takes ~30 seconds to build in debug builds
//! (~1-3 s in release). This module builds it once per process and
//! caches it for the process lifetime.
//!
//! # Preparation
//!
//! [`CachedOrchardProver::prepare`] is the awaitable entry point: every
//! concurrent caller shares ONE build, which runs on tokio's blocking
//! pool (never on an async worker), and the call returns once the key is
//! ready. The shielded send operations await the same shared
//! preparation (through [`ShieldedProver::ready`]) before proving, so a
//! send that arrives while a warm-up is still running waits only for the
//! remaining build time and parks no async worker while it does.
//!
//! [`CachedOrchardProver::warm_up`] is the legacy blocking entry point;
//! it joins the same build (a concurrent async preparation and a
//! blocking warm-up never build the key twice).
//!
//! # Proving off the async runtime
//!
//! Proof generation itself is CPU-bound for seconds.
//! [`prove_on_blocking_thread`] waits for the prover to be ready and
//! then hands the proving closure to a tokio blocking-pool thread, so the
//! async workers stay free for networking and shielded sync. On Apple
//! targets that blocking thread only waits: the closure itself runs on a
//! dedicated user-initiated-QoS rayon pool (see `on_proving_threads`);
//! elsewhere it runs on the blocking thread.

use std::sync::OnceLock;

use async_trait::async_trait;
use dpp::shielded::builder::OrchardProver;
use grovedb_commitment_tree::ProvingKey;

use crate::error::PlatformWalletError;

/// A lazily-built, process-lifetime value whose build is expensive and
/// blocking, with a shared async preparation path.
///
/// Exactly one build ever runs, no matter how many sync
/// ([`get_or_build_blocking`](Self::get_or_build_blocking)) and async
/// ([`prepare`](Self::prepare)) callers race: the value lives in a
/// `std::sync::OnceLock`, whose initializer runs at most once while every
/// other caller waits for it. Async callers additionally coalesce on a
/// `tokio::sync::OnceCell`, so concurrent `prepare` calls park as tasks
/// on one blocking-pool job instead of each occupying a thread.
///
/// The build function is a parameter so tests can count builds and
/// control their duration without paying for the real Halo 2 key; so is
/// the runner that executes it (the real key builds on the proving threads,
/// see [`on_proving_threads`]; test cells build inline so they never queue
/// behind a real key build in the same test binary).
pub(crate) struct ProvingKeyCell<K: 'static> {
    value: OnceLock<K>,
    async_init: tokio::sync::OnceCell<()>,
    build: fn() -> K,
    run: fn(fn() -> K) -> K,
}

impl<K: Send + Sync + 'static> ProvingKeyCell<K> {
    pub(crate) const fn new(build: fn() -> K, run: fn(fn() -> K) -> K) -> Self {
        Self {
            value: OnceLock::new(),
            async_init: tokio::sync::OnceCell::const_new(),
            build,
            run,
        }
    }

    /// The value, if it has already been built.
    pub(crate) fn get(&self) -> Option<&K> {
        self.value.get()
    }

    /// The value, building it if necessary. Blocks the calling thread for
    /// the whole build (or until a build already running elsewhere
    /// finishes).
    ///
    /// Invariant: never call this from an async context, and never from a
    /// rayon worker (of the proving pool or the global pool) while the
    /// value is unbuilt. Such a worker waiting inside the build can steal
    /// another job that also waits on this cell from the same thread — a
    /// deadlock. Proving code therefore always awaits
    /// [`ShieldedProver::ready`] before entering the proving pool, and the
    /// ready prover never touches this cell.
    pub(crate) fn get_or_build_blocking(&self) -> &K {
        if let Some(value) = self.value.get() {
            return value;
        }
        #[cfg(target_vendor = "apple")]
        debug_assert!(
            !apple_qos::on_proving_pool_thread(),
            "the proving key must be prepared before proving on the proving pool"
        );
        let (build, run) = (self.build, self.run);
        self.value.get_or_init(|| run(build))
    }

    /// Wait until the value is built, building it on tokio's blocking
    /// pool if nobody has started yet. Concurrent callers share one
    /// build. Returns immediately once the value exists.
    ///
    /// Outside a tokio runtime there is no blocking pool to hand the
    /// build to, so the calling thread blocks for it (as in
    /// [`get_or_build_blocking`](Self::get_or_build_blocking)).
    ///
    /// Errors only if the build panicked or the runtime is shutting down;
    /// the cell then stays empty and a later call retries. Note the retry
    /// granularity (`OnceCell::get_or_try_init` semantics): a persistently
    /// failing build is re-attempted once by each waiter queued behind the
    /// failed attempt, one after another.
    pub(crate) async fn prepare(&'static self) -> Result<&'static K, PlatformWalletError> {
        if let Some(value) = self.value.get() {
            return Ok(value);
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return Ok(self.get_or_build_blocking());
        }
        self.async_init
            .get_or_try_init(|| async move {
                // If this future is dropped mid-build, the next waiter
                // re-runs this initializer: its blocking job then waits on
                // the `OnceLock` behind the first job instead of building a
                // second key.
                tokio::task::spawn_blocking(move || {
                    self.get_or_build_blocking();
                })
                .await
                .map_err(|e| {
                    let reason = if e.is_panic() {
                        "the build panicked"
                    } else {
                        "the build task was cancelled"
                    };
                    PlatformWalletError::ShieldedBuildError(format!(
                        "Orchard proving key preparation failed: {reason}"
                    ))
                })
            })
            .await?;
        Ok(self
            .value
            .get()
            .expect("the shared preparation completed, so the value is built"))
    }
}

/// Run CPU-heavy Orchard work (the proving-key build or a proof) on the
/// threads reserved for it, blocking the caller until it finishes.
///
/// On Apple targets this is a dedicated rayon pool whose threads request
/// the `USER_INITIATED` QoS class at start-up and have the same 8 MiB stack
/// as the FFI runtime's threads (Halo 2 circuit synthesis recurses deeply).
/// Darwin threads otherwise inherit the QoS of whichever thread created
/// them — tokio blocking-pool threads are spawned lazily by whatever thread
/// first needs one, and the process-global rayon pool (which halo2 uses for
/// its parallel FFTs/MSMs, and which dash-spv also uses) by whatever thread
/// first touches rayon — so a warm-up kicked off from a background-priority
/// host task could otherwise leave the build, and every later proof's
/// parallel work, throttled at background QoS. Running inside
/// `ThreadPool::install` also makes halo2's nested rayon parallelism use
/// this pool rather than the global one.
///
/// Top-level jobs enter the pool one at a time. A single proof (or key
/// build) already saturates the pool; letting two in at once would let a
/// worker that is blocked in a `join` inside proof A steal proof B's whole
/// top-level job and run it on A's stack, so A would finish only after
/// A + B and both would share one 8 MiB stack. A call made from a pool
/// thread is already on the proving threads and runs inline (taking the
/// gate again would deadlock).
///
/// Elsewhere (and if the pool cannot be created) the work runs inline.
///
/// Private on purpose: callers must have the proving key built first (see
/// [`ProvingKeyCell::get_or_build_blocking`]'s invariant), which
/// [`prove_on_blocking_thread`] guarantees by awaiting `ready()`.
fn on_proving_threads<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    #[cfg(target_vendor = "apple")]
    {
        if apple_qos::on_proving_pool_thread() {
            return work();
        }
        if let Some(pool) = apple_qos::proving_pool() {
            let _one_job_at_a_time = apple_qos::PROVING_GATE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            return pool.install(work);
        }
    }
    work()
}

#[cfg(target_vendor = "apple")]
mod apple_qos {
    use std::sync::{Mutex, OnceLock};

    /// Serializes top-level jobs entering the proving pool (see
    /// `on_proving_threads`).
    pub(super) static PROVING_GATE: Mutex<()> = Mutex::new(());

    /// Same stack as the FFI runtime's workers (`WORKER_STACK_BYTES`).
    const PROVING_THREAD_STACK_BYTES: usize = 8 * 1024 * 1024;

    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();

    pub(super) fn proving_pool() -> Option<&'static rayon::ThreadPool> {
        POOL.get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .thread_name(|i| format!("orchard-prover-{i}"))
                .stack_size(PROVING_THREAD_STACK_BYTES)
                .start_handler(|_| request_user_initiated_qos())
                .build()
                .ok()
        })
        .as_ref()
    }

    /// Whether the current thread is one of the proving pool's (without
    /// creating the pool).
    pub(super) fn on_proving_pool_thread() -> bool {
        POOL.get()
            .and_then(Option::as_ref)
            .is_some_and(|pool| pool.current_thread_index().is_some())
    }

    /// Request the user-initiated QoS class for the current thread. Best
    /// effort: failure leaves the thread at its inherited QoS.
    fn request_user_initiated_qos() {
        // SAFETY: plain FFI call on the current thread with a valid class.
        unsafe {
            libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INITIATED, 0);
        }
    }
}

/// Global proving key cache — built once, shared for the process lifetime.
static PROVING_KEY: ProvingKeyCell<ProvingKey> =
    ProvingKeyCell::new(ProvingKey::build, on_proving_threads);

/// A cached Orchard prover that lazily builds and caches the Halo 2
/// proving key.
///
/// This struct is zero-sized — all state lives in a process-global cell.
/// Multiple `CachedOrchardProver` instances share the same cached key.
///
/// Implements [`OrchardProver`] (via a `&CachedOrchardProver` reference) so
/// it can be passed directly to DPP's `build_*_transition()` builders, and
/// [`ShieldedProver`] so the wallet's send operations can await the
/// shared preparation and prove off the async runtime.
#[derive(Debug, Clone, Copy, Default)]
pub struct CachedOrchardProver;

impl CachedOrchardProver {
    /// Create a new prover handle.
    ///
    /// This does **not** build the proving key — call
    /// [`prepare`](Self::prepare) / [`warm_up`](Self::warm_up) or wait for
    /// the first proof generation to trigger the build.
    pub fn new() -> Self {
        CachedOrchardProver
    }

    /// Wait until the proving key is built, building it on tokio's
    /// blocking pool if no build is running yet. Concurrent callers (and
    /// a concurrent blocking [`warm_up`](Self::warm_up)) share one build;
    /// no async worker is blocked while waiting. Returns immediately once
    /// the key is cached.
    pub async fn prepare() -> Result<(), PlatformWalletError> {
        PROVING_KEY.prepare().await.map(|_| ())
    }

    /// Build the proving key if it hasn't been built yet.
    ///
    /// This is a blocking operation (up to ~30 seconds on first call).
    /// Subsequent calls return immediately. Call this on a background
    /// thread, never on an async worker — async code should await
    /// [`prepare`](Self::prepare) instead.
    pub fn warm_up(&self) {
        let _ = PROVING_KEY.get_or_build_blocking();
    }

    /// Whether the proving key has already been built and cached.
    pub fn is_ready(&self) -> bool {
        PROVING_KEY.get().is_some()
    }

    /// Get a reference to the cached proving key, building it if necessary.
    fn get_or_build(&self) -> &'static ProvingKey {
        PROVING_KEY.get_or_build_blocking()
    }
}

impl OrchardProver for &CachedOrchardProver {
    fn proving_key(&self) -> &ProvingKey {
        self.get_or_build()
    }
}

/// An [`OrchardProver`] whose proving key is already built.
///
/// `Copy + Send + Sync + 'static`, so it can move into a blocking-pool
/// closure. Obtained from [`ShieldedProver::ready`].
#[derive(Clone, Copy)]
pub struct ReadyOrchardProver(&'static ProvingKey);

impl OrchardProver for ReadyOrchardProver {
    fn proving_key(&self) -> &ProvingKey {
        self.0
    }
}

/// Source of a ready-to-use Orchard prover for the wallet's shielded
/// send operations.
///
/// The operations await [`ready`](Self::ready) — which for
/// [`CachedOrchardProver`] is the shared, off-runtime proving-key
/// preparation — and then move the returned prover onto a blocking
/// thread for proof generation (see [`prove_on_blocking_thread`]).
#[async_trait]
pub trait ShieldedProver: Send + Sync {
    /// The prover handed to the DPP builders once the key is ready.
    type Ready: OrchardProver + Send + Sync + 'static;

    /// Wait until the prover is ready (building its key if necessary,
    /// without blocking an async worker) and return it.
    async fn ready(&self) -> Result<Self::Ready, PlatformWalletError>;
}

#[async_trait]
impl ShieldedProver for CachedOrchardProver {
    type Ready = ReadyOrchardProver;

    async fn ready(&self) -> Result<ReadyOrchardProver, PlatformWalletError> {
        PROVING_KEY.prepare().await.map(ReadyOrchardProver)
    }
}

#[async_trait]
impl<T: ShieldedProver + ?Sized> ShieldedProver for &T {
    type Ready = T::Ready;

    async fn ready(&self) -> Result<T::Ready, PlatformWalletError> {
        (**self).ready().await
    }
}

/// Wait for `prover` to be ready, then run `prove` with it off the async
/// runtime: a tokio blocking-pool thread hands it to the proving threads
/// (see [`on_proving_threads`]) and waits.
///
/// Everything `prove` needs must be moved in (`Send + 'static`); it gets
/// the ready prover by reference, matching the DPP builders' `&P`. A panic
/// in `prove` is re-raised to the caller.
pub(crate) async fn prove_on_blocking_thread<P, T, F>(
    prover: &P,
    prove: F,
) -> Result<T, PlatformWalletError>
where
    P: ShieldedProver + ?Sized,
    F: FnOnce(&P::Ready) -> T + Send + 'static,
    T: Send + 'static,
{
    // `ready()` first: the key must be built before any work enters the
    // proving pool (a lazy build from a pool worker could deadlock, see
    // `ProvingKeyCell::get_or_build_blocking`).
    let ready = prover.ready().await?;
    let work = move || on_proving_threads(move || prove(&ready));
    // Outside a tokio runtime there is no blocking pool: run synchronously.
    if tokio::runtime::Handle::try_current().is_err() {
        return Ok(work());
    }
    match tokio::task::spawn_blocking(work).await {
        Ok(value) => Ok(value),
        // Re-raise a proving panic exactly as if it had run on this thread.
        Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
        Err(_) => Err(PlatformWalletError::ShieldedBuildError(
            "proof generation was cancelled (runtime shutting down)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn test_prover_starts_not_ready() {
        // Note: if another test already warmed up the key in this process,
        // this will pass trivially. That's fine — we just verify the API works.
        let prover = CachedOrchardProver::new();
        let _ = prover.is_ready();
    }

    /// Test cells build on the calling thread, never on the shared proving
    /// pool, so their timing can't queue behind a real key build running in
    /// another test of this binary.
    fn run_inline<K>(build: fn() -> K) -> K {
        build()
    }

    /// Each test owns its own static cell + counter so tests can run in
    /// parallel without sharing build state.
    macro_rules! test_cell {
        ($cell:ident, $builds:ident, $millis:expr) => {
            static $builds: AtomicUsize = AtomicUsize::new(0);
            static $cell: ProvingKeyCell<u64> = ProvingKeyCell::new(
                || {
                    $builds.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis($millis));
                    42
                },
                run_inline,
            );
        };
    }

    /// A cell whose build blocks until the test sets `$release`, so tests
    /// can observe "build in flight" deterministically instead of racing
    /// wall-clock sleeps (CI machines are noisy). The deadline only turns
    /// a broken test into a failure instead of a hang.
    macro_rules! gated_cell {
        ($cell:ident, $builds:ident, $release:ident) => {
            static $builds: AtomicUsize = AtomicUsize::new(0);
            static $release: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            static $cell: ProvingKeyCell<u64> = ProvingKeyCell::new(
                || {
                    $builds.fetch_add(1, Ordering::SeqCst);
                    let deadline = Instant::now() + HANG_GUARD;
                    while !$release.load(Ordering::SeqCst) {
                        assert!(Instant::now() < deadline, "build never released");
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    42
                },
                run_inline,
            );
        };
    }

    /// Upper bound for "this must eventually happen"; never a latency
    /// threshold.
    const HANG_GUARD: Duration = Duration::from_secs(120);

    /// Yield to the runtime until `cond` holds (bounded by `HANG_GUARD`).
    async fn wait_until(cond: impl Fn() -> bool) {
        let deadline = Instant::now() + HANG_GUARD;
        while !cond() {
            assert!(Instant::now() < deadline, "condition never became true");
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_prepare_callers_share_one_build() {
        test_cell!(CELL, BUILDS, 200);
        let handles: Vec<_> = (0..32)
            .map(|_| tokio::spawn(async { CELL.prepare().await.map(|v| v as *const u64 as usize) }))
            .collect();
        let mut ptrs = Vec::new();
        for h in handles {
            ptrs.push(h.await.unwrap().unwrap());
        }
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1, "exactly one build");
        assert!(
            ptrs.windows(2).all(|w| w[0] == w[1]),
            "all callers see one value"
        );
        // A later call reuses the cached value.
        assert_eq!(*CELL.prepare().await.unwrap(), 42);
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn blocking_warm_up_and_async_prepare_share_one_build() {
        test_cell!(CELL, BUILDS, 300);
        // Legacy warm-up: a plain thread blocking in the sync path.
        let warm = std::thread::spawn(|| *CELL.get_or_build_blocking());
        tokio::time::sleep(Duration::from_millis(50)).await;
        let prepared = CELL.prepare().await.unwrap();
        assert_eq!(*prepared, 42);
        assert_eq!(warm.join().unwrap(), 42);
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    /// The FFI runtime is multi-threaded, but the wallet may be driven
    /// from a current-thread runtime (tests, other hosts). There the
    /// single runtime thread must keep polling other tasks while the key
    /// builds — i.e. the build must not run on the runtime thread. If it
    /// did, the runtime could never get back to this test body to release
    /// the build, and the build's hang guard would fail the test.
    #[tokio::test(flavor = "current_thread")]
    async fn prepare_on_current_thread_runtime_keeps_runtime_responsive() {
        gated_cell!(CELL, BUILDS, RELEASE);
        let prepare = tokio::spawn(CELL.prepare());
        wait_until(|| BUILDS.load(Ordering::SeqCst) == 1).await;
        // The build is in flight and the runtime thread is still free.
        for _ in 0..3 {
            tokio::task::yield_now().await;
        }
        RELEASE.store(true, Ordering::SeqCst);
        assert_eq!(*prepare.await.unwrap().unwrap(), 42);
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    /// A send that arrives while a warm-up is building joins that build:
    /// it does not start another, and it completes as soon as the build
    /// does (it only waits for the remainder).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn send_started_mid_warm_up_waits_for_the_in_flight_build() {
        gated_cell!(CELL, BUILDS, RELEASE);
        let warm = tokio::spawn(CELL.prepare());
        wait_until(|| BUILDS.load(Ordering::SeqCst) == 1).await;
        let send = tokio::spawn(CELL.prepare());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!send.is_finished(), "the send must wait for the key");
        let released = Instant::now();
        RELEASE.store(true, Ordering::SeqCst);
        assert_eq!(*send.await.unwrap().unwrap(), 42);
        eprintln!(
            "send completed {:?} after the in-flight build finished",
            released.elapsed()
        );
        warm.await.unwrap().unwrap();
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    /// Aborting the waiter that started the build must not start a second
    /// build: the next waiter re-runs the async initializer, whose blocking
    /// job just waits on the `OnceLock` behind the first one.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn aborted_prepare_waiter_does_not_cause_a_second_build() {
        gated_cell!(CELL, BUILDS, RELEASE);
        let first = tokio::spawn(CELL.prepare());
        wait_until(|| BUILDS.load(Ordering::SeqCst) == 1).await;
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        let second = tokio::spawn(CELL.prepare());
        tokio::time::sleep(Duration::from_millis(20)).await;
        RELEASE.store(true, Ordering::SeqCst);
        assert_eq!(*second.await.unwrap().unwrap(), 42);
        assert_eq!(*CELL.prepare().await.unwrap(), 42);
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn panicking_build_reports_error_and_a_later_call_retries() {
        static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
        static CELL: ProvingKeyCell<u64> = ProvingKeyCell::new(
            || {
                if ATTEMPTS.fetch_add(1, Ordering::SeqCst) == 0 {
                    panic!("first build fails");
                }
                7
            },
            run_inline,
        );
        let err = CELL.prepare().await.unwrap_err();
        assert!(err.to_string().contains("panicked"), "{err}");
        assert!(CELL.get().is_none());
        assert_eq!(*CELL.prepare().await.unwrap(), 7);
        assert_eq!(ATTEMPTS.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn prepare_without_runtime_builds_on_the_calling_thread() {
        test_cell!(CELL, BUILDS, 10);
        let value = futures::executor::block_on(CELL.prepare()).unwrap();
        assert_eq!(*value, 42);
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    /// Test double for the send path: a `ShieldedProver` backed by a test
    /// cell. Its `Ready` reuses the real key type only nominally — the
    /// closures under test never touch the key.
    struct CountingProver(&'static ProvingKeyCell<u64>);

    #[derive(Clone, Copy)]
    struct CountingReady(&'static u64);

    impl OrchardProver for CountingReady {
        fn proving_key(&self) -> &ProvingKey {
            unreachable!("send-path tests never prove for real")
        }
    }

    #[async_trait]
    impl ShieldedProver for CountingProver {
        type Ready = CountingReady;
        async fn ready(&self) -> Result<CountingReady, PlatformWalletError> {
            self.0.prepare().await.map(CountingReady)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn send_path_awaits_the_shared_preparation() {
        test_cell!(CELL, BUILDS, 300);
        let prover = CountingProver(&CELL);
        let warm = tokio::spawn(CELL.prepare());
        let sends: Vec<_> = (0..4)
            .map(|i| {
                let prover = CountingProver(&CELL);
                tokio::spawn(async move {
                    prove_on_blocking_thread(&prover, move |ready| *ready.0 + i)
                        .await
                        .unwrap()
                })
            })
            .collect();
        let direct = prove_on_blocking_thread(&&prover, |ready| *ready.0)
            .await
            .unwrap();
        assert_eq!(direct, 42);
        for (i, s) in sends.into_iter().enumerate() {
            assert_eq!(s.await.unwrap(), 42 + i as u64);
        }
        warm.await.unwrap().unwrap();
        assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
    }

    /// On a current-thread runtime, the runtime keeps running other tasks
    /// while a proof is in progress: the "proof" only finishes once a task
    /// on the same runtime has ticked. Inline proving (the pre-change
    /// behavior) would block the runtime thread, the ticker would never
    /// run, and the proof would report the timeout. No latency thresholds;
    /// see the ignored real-proof test for measurements.
    #[tokio::test(flavor = "current_thread")]
    async fn proving_off_runtime_keeps_runtime_responsive() {
        test_cell!(CELL, BUILDS, 0);
        CELL.prepare().await.unwrap();
        let prover = CountingProver(&CELL);
        let (ticked_tx, ticked_rx) = std::sync::mpsc::channel::<()>();
        let ticker = async move {
            for _ in 0..3 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            ticked_tx.send(()).unwrap();
        };
        let proof =
            prove_on_blocking_thread(&prover, move |_| ticked_rx.recv_timeout(HANG_GUARD).is_ok());
        let ((), runtime_ticked_during_proof) = tokio::join!(ticker, proof);
        assert!(runtime_ticked_during_proof.unwrap());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn panic_while_proving_propagates() {
        test_cell!(CELL, BUILDS, 0);
        let prover = CountingProver(&CELL);
        let joined = tokio::spawn(async move {
            prove_on_blocking_thread(&prover, |_| -> u8 { panic!("prover bug") }).await
        })
        .await;
        assert!(joined.unwrap_err().is_panic());
    }

    /// Real proving key: concurrent `prepare` callers share one build and
    /// the key is usable afterwards. Slow (~30 s debug, ~1-3 s release),
    /// so ignored by default:
    /// `cargo test -p platform-wallet --features shielded --release -- --ignored real_proving_key`
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "builds the real Halo 2 proving key"]
    async fn real_proving_key_prepare_is_shared_and_timed() {
        let start = Instant::now();
        let handles: Vec<_> = (0..8)
            .map(|_| tokio::spawn(CachedOrchardProver::prepare()))
            .collect();
        for h in handles {
            h.await.unwrap().unwrap();
        }
        let built = start.elapsed();
        assert!(CachedOrchardProver::new().is_ready());
        let ready = CachedOrchardProver.ready().await.unwrap();
        let _ = ready.proving_key();
        eprintln!("real proving key prepared in {built:?} (8 concurrent callers)");
    }

    /// Darwin QoS: even when the build is requested from a
    /// background-QoS thread, the build and its nested rayon work run at
    /// user-initiated QoS on the dedicated proving pool.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn proving_work_runs_at_user_initiated_qos_on_apple() {
        fn current_qos() -> u32 {
            let mut class: u32 = 0;
            let mut relative: libc::c_int = 0;
            // SAFETY: valid out-pointers; `qos_class_t` is a `u32` enum and
            // is read back as a raw `u32` so no unknown discriminant is
            // ever materialized as the enum.
            unsafe {
                libc::pthread_get_qos_class_np(
                    libc::pthread_self(),
                    &mut class as *mut u32 as *mut libc::qos_class_t,
                    &mut relative,
                );
            }
            class
        }
        const BACKGROUND: u32 = libc::qos_class_t::QOS_CLASS_BACKGROUND as u32;
        const USER_INITIATED: u32 = libc::qos_class_t::QOS_CLASS_USER_INITIATED as u32;

        let observed = std::thread::spawn(|| {
            // SAFETY: plain FFI call on the current thread.
            unsafe {
                libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_BACKGROUND, 0);
            }
            assert_eq!(current_qos(), BACKGROUND);
            let (outer, nested) = on_proving_threads(|| {
                let (a, b) = rayon::join(current_qos, current_qos);
                assert!(rayon::current_thread_index().is_some());
                (current_qos(), [a, b])
            });
            // The caller's own QoS is untouched.
            assert_eq!(current_qos(), BACKGROUND);
            (outer, nested)
        })
        .join()
        .unwrap();
        assert_eq!(observed.0, USER_INITIATED);
        assert_eq!(observed.1, [USER_INITIATED, USER_INITIATED]);
    }

    /// Top-level jobs enter the Apple proving pool one at a time, and a
    /// nested call from a pool thread runs inline instead of deadlocking
    /// on the gate.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn proving_pool_runs_one_top_level_job_at_a_time() {
        static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
        static MAX_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
        let job = || {
            on_proving_threads(|| {
                let now = IN_FLIGHT.fetch_add(1, Ordering::SeqCst) + 1;
                MAX_IN_FLIGHT.fetch_max(now, Ordering::SeqCst);
                // Nested parallel work, as halo2 does, plus a re-entrant call.
                let (a, b) = rayon::join(|| 1, || on_proving_threads(|| 2));
                std::thread::sleep(Duration::from_millis(20));
                IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
                a + b
            })
        };
        let threads: Vec<_> = (0..4).map(|_| std::thread::spawn(job)).collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), 3);
        }
        assert_eq!(MAX_IN_FLIGHT.load(Ordering::SeqCst), 1);
    }
}
