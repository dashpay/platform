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

        /** One-time native library load + `dash_sdk_init`. Idempotent. */
        fun initialize() = NativeLoader.ensureLoaded()

        /** Enable console (logcat) logging for SDK operations. */
        fun enableLogging(level: LogLevel = LogLevel.DEBUG) {
            initialize()
            SdkNative.enableLogging(level.value)
        }

        /**
         * Route the global tracing subscriber to per-bucket files under
         * [sessionRoot]. Returns false if a subscriber was already
         * installed or the path is unwritable.
         */
        fun enableFileLogging(level: LogLevel = LogLevel.DEBUG, sessionRoot: String): Boolean {
            initialize()
            return SdkNative.enableFileLogging(level.value, sessionRoot)
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
