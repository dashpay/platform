package org.dashfoundation.dashsdk

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.NativeCleaner
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import java.io.File
import java.net.HttpURLConnection
import java.net.URI
import java.util.concurrent.atomic.AtomicLong

/**
 * Kotlin wrapper for the Dash Platform SDK — port of
 * `SwiftDashSDK/SDK.swift`.
 *
 * One instance wraps one native `SDKHandle`, locked to a [network] at
 * creation. Network switching means closing this instance and creating a
 * new one (see `WalletManagerStore`).
 *
 * Instances are created with [create]; the handle is released by [close]
 * (owners should use `use {}` or tie it to a lifecycle), with a [NativeCleaner]
 * backstop for leaked instances.
 */
class Sdk private constructor(
    handle: Long,
    val network: Network,
    // Test-only observability: runs exactly once when the cleanup fires
    // (lifecycle tests assert cleanup executed despite cancellation).
    onCleanup: (() -> Unit)? = null,
) : AutoCloseable {

    private val handleRef = AtomicLong(handle)

    private val cleanable =
        NativeCleaner.register(this, HandleCleanup(handleRef, onCleanup))

    /** Identity queries — mirrors Swift's `sdk.identities`. */
    val identities: org.dashfoundation.dashsdk.queries.Identities by lazy {
        org.dashfoundation.dashsdk.queries.Identities(this)
    }

    /** DPNS name queries. */
    val dpns: org.dashfoundation.dashsdk.queries.Dpns by lazy {
        org.dashfoundation.dashsdk.queries.Dpns(this)
    }

    /** Data contract queries. */
    val contracts: org.dashfoundation.dashsdk.queries.Contracts by lazy {
        org.dashfoundation.dashsdk.queries.Contracts(this)
    }

    /** Document queries. */
    val documents: org.dashfoundation.dashsdk.queries.Documents by lazy {
        org.dashfoundation.dashsdk.queries.Documents(this)
    }

    /** Read-only token queries (balances, statuses, contract info, id calc). */
    val tokenQueries: org.dashfoundation.dashsdk.queries.TokenQueries by lazy {
        org.dashfoundation.dashsdk.queries.TokenQueries(this)
    }

    /** Platform address queries (balance + nonce). */
    val addresses: org.dashfoundation.dashsdk.queries.Addresses by lazy {
        org.dashfoundation.dashsdk.queries.Addresses(this)
    }

    /** Contested-resource and voting read queries. */
    val voting: org.dashfoundation.dashsdk.queries.Voting by lazy {
        org.dashfoundation.dashsdk.queries.Voting(this)
    }

    /** Evonode proposed-epoch-block queries. */
    val evonodes: org.dashfoundation.dashsdk.queries.Evonodes by lazy {
        org.dashfoundation.dashsdk.queries.Evonodes(this)
    }

    /** System / protocol / epoch queries (GroveDB, credits, quorums). */
    val system: org.dashfoundation.dashsdk.queries.SystemQueries by lazy {
        org.dashfoundation.dashsdk.queries.SystemQueries(this)
    }

    /** Group (multi-party control) read queries. */
    val groups: org.dashfoundation.dashsdk.queries.Groups by lazy {
        org.dashfoundation.dashsdk.queries.Groups(this)
    }

    /**
     * The raw native handle for FFI calls. Throws if the SDK was closed —
     * matching the Swift SDK where a nil handle is a programmer error.
     */
    val handle: Long
        get() = handleRef.get().also {
            check(it != 0L) { "SDK instance has been closed" }
        }

    val isClosed: Boolean get() = handleRef.get() == 0L

    /**
     * Lease fence for in-flight queries: every [org.dashfoundation.dashsdk.queries]
     * entry point runs under it, and [closeSuspending] awaits it before
     * freeing the native SDK (`dash_sdk_destroy` is a raw, non-refcounted
     * `Box::from_raw`). Same primitive as the wallet manager's teardown
     * fence.
     */
    internal val queryGate = org.dashfoundation.dashsdk.wallet.TeardownGate()

    /**
     * Suspending close — the production path (`AppState.initializeSdk`
     * replaces the SDK on a network switch): rejects new queries, awaits
     * every in-flight one, then destroys the native handle. Without this,
     * a query mid-JNI-call could keep dereferencing the freed pointer.
     */
    suspend fun closeSuspending() {
        // NonCancellable for the same reason as PlatformWalletManager's
        // teardown: closeAndAwait is a cancellable suspension, and a caller
        // cancelled mid-close would otherwise skip cleanable.clean() —
        // stranding the native SDK until the GC-driven Cleaner backstop.
        kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) {
            queryGate.closeAndAwait()
            cleanable.clean()
        }
    }

    /**
     * Blocking close for non-suspend contexts (tests, `use {}`). Delegates
     * through [closeSuspending] so even this path awaits in-flight leased
     * queries before the native free — mirroring
     * `PlatformWalletManager.close()`. Never call on the main thread.
     * (The [NativeCleaner] backstop still frees the raw handle directly
     * when the instance is GC'd unreferenced.)
     */
    override fun close() {
        kotlinx.coroutines.runBlocking { closeSuspending() }
    }

    /** Runs on [NativeCleaner] or [close]; destroys the handle exactly once. */
    private class HandleCleanup(
        private val handleRef: AtomicLong,
        private val onCleanup: (() -> Unit)? = null,
    ) : Runnable {
        override fun run() {
            val handle = handleRef.getAndSet(0)
            if (handle != 0L) {
                SdkNative.destroy(handle)
            }
            onCleanup?.invoke()
        }
    }

    enum class LogLevel(val value: Int) {
        ERROR(0), WARN(1), INFO(2), DEBUG(3), TRACE(4)
    }

    /**
     * Outcome of [installFileLogging]. The native installer returns a
     * single boolean whose `false` conflates its two failure modes —
     * field logs showed exactly that: "NOT installed (subscriber already
     * set or dir unwritable)" with no way to tell which. This type keeps
     * them apart so the failure is diagnosable from a log line.
     */
    enum class FileLoggingInstall {
        /** This call installed the file-logging subscriber. */
        INSTALLED,

        /**
         * A process-global tracing subscriber was already set (first init
         * wins — e.g. console logging via [enableLogging] ran first, or
         * another in-process library installed one). The `tracing` API has
         * no way to attach layers to an already-installed subscriber, so
         * file logging cannot be added after the fact; install file
         * logging FIRST if both are wanted.
         */
        ALREADY_SET,

        /**
         * The session root could not be created or written — file logging
         * was not attempted (the native installer would have failed on the
         * same directory). The path is named in the logged warning.
         */
        SESSION_ROOT_UNWRITABLE,
    }

    @Serializable
    private data class MasternodesEnvelope(
        val success: Boolean,
        val data: List<MasternodeEntry> = emptyList(),
    )

    /**
     * One `/masternodes` entry. Wire keys are camelCase; optional fields
     * mirror the Swift decoder's tolerance (`SDK.swift`
     * `discoverActiveMasternodes`) — a single strict field would fail the
     * whole devnet discovery.
     */
    // `internal`, not public: this is a wire-format DTO for the `/masternodes`
    // response, touched only by the `private` [MasternodesEnvelope] and
    // `internal` [parseActiveMasternodes] — never returned by a public function
    // (public discovery returns the non-@Serializable [ActiveMasternode]). Keeping
    // it out of the public ABI means its generated `serializer()` isn't public
    // API, so consumers of the published coordinate don't need
    // kotlinx-serialization on their compile classpath and it can stay
    // `implementation`.
    @Serializable
    internal data class MasternodeEntry(
        val address: String,
        val status: String = "",
        val platformHTTPPort: Int? = null,
        val versionCheck: String? = null,
    )

    /** A discovered active masternode: SPV peer address + DAPI URL. */
    data class ActiveMasternode(val spvPeer: String, val dapiUrl: String)

    companion object {
        /**
         * Native-free instance for JVM lifecycle tests: handle 0 makes
         * [HandleCleanup] skip `SdkNative.destroy`, so the queryGate /
         * closeSuspending semantics are testable without the .so loaded.
         */
        internal fun forLifecycleTest(onCleanup: (() -> Unit)? = null): Sdk =
            Sdk(0L, Network.TESTNET, onCleanup)

        private val json = Json { ignoreUnknownKeys = true }

        private const val LOG_TAG = "DashSdk"

        /** One-time native library load + `dash_sdk_init`. Idempotent. */
        fun initialize() = NativeLoader.ensureLoaded()

        /**
         * Best-effort record of which call claimed the process-global
         * tracing subscriber, for the [ALREADY_SET][FileLoggingInstall.ALREADY_SET]
         * diagnostic. In-process bookkeeping only — a subscriber installed
         * outside this companion (another library) is invisible here, so a
         * `null` value means "not through this API", not "none".
         */
        @Volatile
        private var subscriberClaimedBy: String? = null

        /** Enable console (logcat) logging for SDK operations. */
        fun enableLogging(level: LogLevel = LogLevel.DEBUG) {
            initialize()
            SdkNative.enableLogging(level.value)
            // First-wins bookkeeping: the global tracing subscriber can only
            // be installed once per process, and console logging installs
            // one — the most common reason a later enableFileLogging finds
            // the slot taken.
            if (subscriberClaimedBy == null) {
                subscriberClaimedBy = "console logging (enableLogging)"
            }
        }

        /**
         * Route the global tracing subscriber to per-bucket files under
         * [sessionRoot]. Returns true only when THIS call installed it —
         * boolean-compat wrapper around [installFileLogging], which callers
         * should prefer: it reports WHICH condition failed (subscriber
         * already set vs. session root unwritable, with the path) instead
         * of an undiagnosable false.
         */
        fun enableFileLogging(level: LogLevel = LogLevel.DEBUG, sessionRoot: String): Boolean =
            installFileLogging(level, sessionRoot) == FileLoggingInstall.INSTALLED

        /**
         * [enableFileLogging] with a diagnosable outcome. On failure the
         * distinguishing condition is also logged as a warning (tag
         * `DashSdk`), so field logs no longer show the ambiguous
         * "NOT installed (subscriber already set or dir unwritable)":
         *
         * - [FileLoggingInstall.SESSION_ROOT_UNWRITABLE] — [sessionRoot]
         *   could not be created/written; checked BEFORE touching the
         *   native installer, and the logged warning names the exact path.
         * - [FileLoggingInstall.ALREADY_SET] — the directory IS writable
         *   but a global tracing subscriber already exists (first init
         *   wins). The `tracing` API cannot re-route an installed
         *   subscriber, so the fix is ordering: install file logging
         *   before [enableLogging]. The warning names the in-process
         *   claimant when it went through this API.
         */
        fun installFileLogging(
            level: LogLevel = LogLevel.DEBUG,
            sessionRoot: String,
        ): FileLoggingInstall {
            val root = File(sessionRoot)
            if (!sessionRootWritable(root)) {
                android.util.Log.w(
                    LOG_TAG,
                    "SDK file logging NOT installed: session root cannot be " +
                        "created/written: ${root.absolutePath}",
                )
                return FileLoggingInstall.SESSION_ROOT_UNWRITABLE
            }
            initialize()
            return if (SdkNative.enableFileLogging(level.value, sessionRoot)) {
                subscriberClaimedBy = "file logging (enableFileLogging)"
                FileLoggingInstall.INSTALLED
            } else {
                val claimant = subscriberClaimedBy
                    ?: "something outside this API (another in-process library, " +
                        "or a subscriber surviving from an earlier init)"
                android.util.Log.w(
                    LOG_TAG,
                    "SDK file logging NOT installed at ${root.absolutePath}: the " +
                        "directory is writable, so a global tracing subscriber was " +
                        "already set — by $claimant. First init wins and the tracing " +
                        "API cannot re-route an installed subscriber; call " +
                        "enableFileLogging before enableLogging to get file logs.",
                )
                FileLoggingInstall.ALREADY_SET
            }
        }

        /**
         * Whether [root] exists (or can be created) as a directory we can
         * write a file into — probed with a real create-and-delete, the
         * same operation the native installer's `open_file` will perform.
         * Factored out (and `internal`) so the pre-native gate is
         * JVM-testable.
         */
        internal fun sessionRootWritable(root: File): Boolean = try {
            root.mkdirs()
            val probe = File(root, ".dash_sdk_write_probe")
            probe.delete()
            val created = probe.createNewFile()
            probe.delete()
            created && root.isDirectory
        } catch (_: Exception) {
            false
        }

        /**
         * Create an SDK instance with trusted setup — port of
         * `SDK.init(network:platformVersion:)` including the override
         * gating and devnet auto-discovery policy:
         *
         * - Regtest: DAPI/quorum overrides applied unconditionally (Rust
         *   has no built-in defaults for regtest).
         * - Devnet: quorum URL required; the DAPI list is ALWAYS
         *   auto-discovered fresh from `{quorumUrl}/masternodes` so the
         *   path self-heals when nodes churn.
         * - Mainnet/testnet: overrides only under [SdkConfig.useDockerSetup];
         *   otherwise Rust picks canonical seeds and quorum endpoints.
         */
        suspend fun create(config: SdkConfig): Sdk = withContext(Dispatchers.IO) {
            initialize()

            val network = config.network
            val useOverrides = network == Network.REGTEST ||
                network == Network.DEVNET ||
                config.useDockerSetup
            val quorumUrl = if (useOverrides) config.quorumUrl?.takeIf { it.isNotEmpty() } else null

            val dapiAddresses: String? = when {
                network == Network.DEVNET ->
                    quorumUrl?.let { discoverDapiAddresses(it, defaultDapiPort(network)) }
                useOverrides ->
                    config.dapiAddresses?.takeIf { it.isNotEmpty() }
                        ?: SdkConfig.DEFAULT_LOCAL_DAPI
                else -> null
            }

            val handle = mapNativeErrors {
                SdkNative.createTrusted(
                    network = network.ffiValue,
                    dapiAddresses = dapiAddresses,
                    quorumUrl = quorumUrl,
                    skipAssetLockProofVerification = config.skipAssetLockProofVerification,
                    requestRetryCount = config.requestRetryCount,
                    requestTimeoutMs = config.requestTimeoutMs,
                    platformVersion = config.platformVersion,
                )
            }
            Sdk(handle, network)
        }

        /**
         * Fetch `{quorumBase}/masternodes` and return active nodes —
         * port of `SDK.discoverActiveMasternodes`. Both the DAPI list and
         * the SPV peer list derive from this. Filters to
         * `status == ENABLED && versionCheck == success` to match the Rust
         * trusted-context provider's active-node policy. Returns null on
         * any failure (timeout, JSON shape mismatch, no active nodes).
         *
         * Compatibility overload retaining the original public JVM descriptor
         * `(Ljava/lang/String;)Ljava/util/List;`. Mainnet's standard HTTPS
         * port remains the legacy default; network-aware callers use the
         * explicit two-argument overload below.
         */
        fun discoverActiveMasternodes(quorumBase: String): List<ActiveMasternode>? =
            discoverActiveMasternodes(quorumBase, 443)

        fun discoverActiveMasternodes(
            quorumBase: String,
            defaultDapiPort: Int,
        ): List<ActiveMasternode>? {
            val base = quorumBase.trimEnd('/')
            val url = try {
                URI("$base/masternodes").toURL()
            } catch (_: Exception) {
                return null
            }
            val body = try {
                val connection = url.openConnection() as HttpURLConnection
                connection.connectTimeout = 5_000
                connection.readTimeout = 5_000
                connection.requestMethod = "GET"
                connection.inputStream.use { it.readBytes().decodeToString() }
            } catch (_: Exception) {
                return null
            }
            return parseActiveMasternodes(body, defaultDapiPort)
        }

        /**
         * Parse a `/masternodes` response body into active nodes. Split
         * from the fetch for testability; filtering rules documented on
         * [discoverActiveMasternodes].
         */
        internal fun parseActiveMasternodes(
            body: String,
            defaultDapiPort: Int = 443,
        ): List<ActiveMasternode>? {
            val envelope = try {
                json.decodeFromString<MasternodesEnvelope>(body)
            } catch (_: Exception) {
                return null
            }
            if (!envelope.success) return null

            val active = envelope.data.mapNotNull { mn ->
                if (mn.status != "ENABLED" || mn.versionCheck != "success") return@mapNotNull null
                val host = endpointHost(mn.address) ?: return@mapNotNull null
                val urlHost = if (':' in host) "[$host]" else host
                val dapiPort = mn.platformHTTPPort ?: defaultDapiPort
                ActiveMasternode(spvPeer = mn.address, dapiUrl = "https://$urlHost:$dapiPort")
            }
            return active.ifEmpty { null }
        }

        private fun discoverDapiAddresses(quorumBase: String, defaultDapiPort: Int): String? =
            discoverActiveMasternodes(quorumBase, defaultDapiPort)
                ?.joinToString(",") { it.dapiUrl }

        private fun defaultDapiPort(network: Network): Int =
            if (network == Network.MAINNET) 443 else 1443

        private fun endpointHost(address: String): String? {
            if (address.startsWith('[')) {
                val closing = address.indexOf(']')
                return address.substring(1, closing.takeIf { it > 1 } ?: return null)
            }
            val separator = address.lastIndexOf(':')
            if (separator <= 0 || ':' in address.substring(0, separator)) return null
            return address.substring(0, separator)
        }

        /** Whether the native library was built with shielded (Orchard) support. */
        fun hasShielded(): Boolean {
            initialize()
            return SdkNative.hasShielded()
        }

        /** Native SDK version string. */
        fun version(): String {
            initialize()
            return SdkNative.version()
        }
    }
}
