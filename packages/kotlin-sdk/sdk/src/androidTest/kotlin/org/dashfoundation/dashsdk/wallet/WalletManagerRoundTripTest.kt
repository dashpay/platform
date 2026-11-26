package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.ffi.VerdictRoundTestHook
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.services.ShieldedService
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.AssumptionViolatedException
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Instrumented round-trip: create a manager, create a wallet from a fixed
 * test mnemonic OFFLINE (the FFI derives locally — `createTrusted` only
 * builds the client, it does not connect), assert Room rows landed via the
 * persistence callbacks, then build a NEW manager against the same DB and
 * [PlatformWalletManager.loadPersistedWallets] to confirm the wallet is
 * restored watch-only.
 *
 * Requires an emulator/device (native lib + Android Keystore). Do NOT run
 * in the JVM unit suite — it is the wallet-manager analog of
 * [org.dashfoundation.dashsdk.FfiSmokeTest]. Orchestrator note: gate this
 * behind `connectedDebugAndroidTest`.
 */
@RunWith(AndroidJUnit4::class)
class WalletManagerRoundTripTest {

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    // BIP39 English test vector (entropy 0x7f repeated).
    private val otherTestMnemonic =
        "legal winner thank year wave sausage worth useful legal winner thank yellow"

    /** How long a test waits for Rust to announce a balance state. */
    private val stateWaitMs = 30_000L

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        // Testnet, no overrides → offline client build (no connection made).
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    @Test
    fun walletCreateAndReloadRoundTrip() = runBlocking {
        val walletId: ByteArray

        // ── First manager: create the wallet from the fixed mnemonic ──
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            val created = manager.createWallet(
                mnemonic = testMnemonic,
                name = "round-trip",
                createDefaultAccounts = true,
            )
            walletId = created.walletId
            assertEquals("wallet id is 32 bytes", 32, walletId.size)

            // The map is keyed by walletId hex.
            assertNotNull(manager.wallet(forWalletId = walletId))

            // Persistence callbacks (fired synchronously from create) must
            // have written the wallet row + at least one account with an xpub.
            val walletRow = db.walletDao().getByWalletId(walletId)
            assertNotNull("wallet row persisted", walletRow)
            assertEquals(Network.TESTNET.ffiValue, walletRow!!.networkRaw)

            val accounts = db.accountDao().observeByWallet(walletId).first()
            assertTrue("at least one account persisted", accounts.isNotEmpty())
            assertTrue(
                "at least one account carries an xpub (restorable)",
                accounts.any { it.accountExtendedPubKeyBytes?.isNotEmpty() == true },
            )

            // The mnemonic must be retrievable keyed by the derived id.
            assertEquals(testMnemonic, walletStorage.retrieveMnemonic(walletId))
        }

        // ── Second manager: reload from persistence (watch-only) ──
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            val restored = reloaded.loadPersistedWallets()
            assertTrue("at least one wallet restored", restored.isNotEmpty())
            val match = reloaded.wallet(forWalletId = walletId)
            assertNotNull("the created wallet is restored by id", match)
            assertTrue(walletId.contentEquals(match!!.walletId))
        }
    }

    /**
     * DashPay persist→wipe→restore→re-read round-trip through the REAL
     * native marshaling (the staging/seal/free pipeline in
     * `rs-unified-sdk-jni/src/persistence.rs` that JVM/Robolectric tests
     * can never reach): inject payment / contact-profile / ignored-sender
     * rows into Room in the shape the persist paths land them, reload
     * the wallet on a fresh manager (the load path marshals the rows into
     * Rust wallet state), then read them back out through the DashPay FFI
     * getters and assert field equality. Wrong lengths / nulls /
     * double-frees in the restore marshaling fail here, not at UAT.
     *
     * A third reload leg covers the tombstone consequence: a tombstoned
     * (deleted) profile row must restore as an ABSENT cache entry — and
     * exercises the empty-contact-profile-array marshaling path.
     *
     * Of the K1 getters, `searchDpnsNames` is deliberately untested here:
     * it is a live network query and belongs to the `-Ptestnet=true`
     * tier.
     */
    @Test
    fun dashPayRestoreRoundTripsPaymentsContactProfilesAndSyncState() = runBlocking {
        val walletId: ByteArray
        val identityId = ByteArray(32) { 42 }
        val contactId = ByteArray(32) { 43 }
        val mutedId = ByteArray(32) { 44 }
        val txid = "ab".repeat(32)

        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            walletId = manager.createWallet(
                mnemonic = testMnemonic,
                name = "dashpay-round-trip",
                createDefaultAccounts = true,
            ).walletId

            // Fixture rows, shaped exactly as the persist paths land them:
            // an identity owned by the wallet, one payment (Sent + memo —
            // the class the reconcile sweep can NOT re-derive), one cached
            // contact profile, one ignored sender.
            db.identityDao().upsert(
                org.dashfoundation.dashsdk.persistence.entities.IdentityEntity(
                    identityId = identityId,
                    balance = 1_000,
                    revision = 1,
                    networkRaw = Network.TESTNET.ffiValue,
                    walletId = walletId,
                    identityIndex = 0,
                ),
            )
            db.dashpayDao().upsertPayments(
                listOf(
                    org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity(
                        networkRaw = Network.TESTNET.ffiValue,
                        ownerIdentityId = identityId,
                        counterpartyIdentityId = contactId,
                        amountDuffs = 777_000,
                        directionRaw = 0, // Sent
                        statusRaw = 1, // Confirmed
                        txid = txid,
                        memo = "round trip",
                    ),
                ),
            )
            db.dashpayDao().upsertContactProfile(
                org.dashfoundation.dashsdk.persistence.entities.DashpayContactProfileEntity(
                    networkRaw = Network.TESTNET.ffiValue,
                    ownerIdentityId = identityId,
                    contactIdentityId = contactId,
                    displayName = "Bob",
                    publicMessage = "yo",
                    avatarUrl = "https://x/bob.png",
                    avatarHash = ByteArray(32) { 23 },
                    avatarFingerprint = null,
                    checkedAtMs = 1_700_000_111_000,
                    // A P2PKH Core address in storage form restores; a
                    // shielded value that is not an Orchard address is
                    // dropped by the wallet at restore.
                    corePaymentAddress = byteArrayOf(0x00) + ByteArray(20) { 0x22 },
                    // pk_d of all 0xff is not a canonical Pallas point.
                    shieldedAddress = ByteArray(43) { 0xff.toByte() },
                ),
            )
            db.dashpayDao().upsertIgnoredSender(
                org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity(
                    networkRaw = Network.TESTNET.ffiValue,
                    ownerIdentityId = identityId,
                    ignoredSenderId = mutedId,
                ),
            )
        }

        // Fresh manager: the load path drives the JNI restore marshaling
        // (build → seal → Rust restore folds → free) for real.
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            reloaded.loadPersistedWallets()
            val managed = reloaded.wallet(forWalletId = walletId)
            assertNotNull("wallet restored", managed)

            val paymentsJson = managed!!.dashpay.payments(identityId)
            assertNotNull("payments restored into Rust state", paymentsJson)
            val payments = org.json.JSONArray(paymentsJson!!)
            assertEquals(1, payments.length())
            val payment = payments.getJSONObject(0)
            assertEquals(txid, payment.getString("txid"))
            assertEquals(contactId.joinToString("") { "%02x".format(it) }, payment.getString("counterpartyId"))
            assertEquals(777_000L, payment.getLong("amountDuffs"))
            assertEquals(0, payment.getInt("direction"))
            assertEquals(1, payment.getInt("status"))
            assertEquals("round trip", payment.getString("memo"))

            val profileJson = managed.dashpay.getContactProfile(identityId, contactId)
            assertNotNull("contact profile restored into Rust cache", profileJson)
            val profile = org.json.JSONObject(profileJson!!)
            assertEquals("Bob", profile.getString("displayName"))
            assertEquals("yo", profile.getString("publicMessage"))
            assertEquals("https://x/bob.png", profile.getString("avatarUrl"))
            assertEquals("17".repeat(32), profile.getString("avatarHash")) // 23 = 0x17
            assertTrue("fingerprint stays absent", !profile.has("avatarFingerprint"))
            assertEquals(
                "Core payment address restored into Rust cache",
                "00" + "22".repeat(20),
                profile.getString("corePaymentAddress"),
            )
            assertTrue("no Platform address stored", !profile.has("platformPaymentAddress"))
            assertTrue("invalid shielded bytes dropped at restore", !profile.has("shieldedAddress"))

            val stateJson = managed.dashpay.syncState(identityId)
            assertNotNull("sync state readable", stateJson)
            val state = org.json.JSONObject(stateJson!!)
            assertEquals(1, state.getInt("dashpayPayments"))
            assertEquals(1, state.getInt("contactProfiles"))
            assertEquals(1, state.getInt("presentContactProfiles"))
            assertEquals(1, state.getInt("ignoredSenders"))

            // Per-account balance snapshot: exercises the
            // AccountBalanceEntryFFI array marshal/free path offline — the
            // freshly-created wallet has default accounts, all zero-balance.
            val balancesJson = reloaded.accountBalances(walletId)
            assertNotNull("account balances readable", balancesJson)
            val balances = org.json.JSONArray(balancesJson!!)
            assertTrue("default accounts present", balances.length() > 0)
            for (i in 0 until balances.length()) {
                val account = balances.getJSONObject(i)
                assertEquals(0L, account.getLong("confirmed"))
                assertEquals(64, account.getString("userIdentityId").length)
            }
        }

        // ── Third reload: a tombstoned (deleted) profile row restores as
        // an ABSENT cache entry, and the empty contact-profile array takes
        // the null/0 marshaling path.
        db.dashpayDao().deleteContactProfile(
            Network.TESTNET.ffiValue,
            identityId,
            contactId,
        )
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { third ->
            third.loadPersistedWallets()
            val managed = third.wallet(forWalletId = walletId)
            assertNotNull(managed)
            assertNull(
                "tombstoned profile must not resurrect",
                managed!!.dashpay.getContactProfile(identityId, contactId),
            )
            val state = org.json.JSONObject(managed.dashpay.syncState(identityId)!!)
            assertEquals(0, state.getInt("contactProfiles"))
            // The other stores are unaffected by the profile tombstone.
            assertEquals(1, state.getInt("dashpayPayments"))
            assertEquals(1, state.getInt("ignoredSenders"))
        }
    }

    /**
     * Create, bind, reload and bind again with a shielded store configured,
     * through JNI and the Room persistence callbacks. With a store
     * configured, each of these calls restores the wallet's private
     * payments, and the native library runs that work on its own
     * big-stack thread: polled on the `Dispatchers.IO` thread that made the
     * call, a wallet create overflowed its stack in a dev-profile library
     * (dashpay/platform#6020 item 5). The persistence callbacks of these
     * calls arrive on that native thread, attached to the JVM for the call.
     *
     * Offline: nothing here contacts the network.
     */
    @Test
    fun walletCreateBindAndReloadRoundTripWithShieldedStore() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "shielded-round-trip-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        val storePath = File(storeDir, "shielded.sqlite").absolutePath
        try {
            val walletId: ByteArray
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(storePath)
                walletId = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-round-trip",
                    createDefaultAccounts = true,
                ).walletId
                assertNotNull("wallet row persisted", db.walletDao().getByWalletId(walletId))
                manager.bindShielded(walletId, listOf(0))
            }

            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
                reloaded.configureShielded(storePath)
                val restored = reloaded.loadPersistedWallets()
                assertTrue("at least one wallet restored", restored.isNotEmpty())
                assertNotNull(reloaded.wallet(forWalletId = walletId))
                reloaded.bindShielded(walletId, listOf(0))
            }
        } finally {
            storeDir.deleteRecursively()
        }
    }

    /**
     * The ordinary shielded accounts of a wallet are read from Rust through
     * JNI, and outlive a Clear: the Clear drops the wallet's registration on
     * the coordinator, not the keys Rust holds for it. Bound again with the
     * list read back, the wallet has the accounts it had before.
     *
     * Starts no shielded sync. It uses this class's SDK fixture like the
     * tests around it.
     */
    @Test
    fun shieldedOrdinaryAccountsOutliveAClearAndBindAgain() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "shielded-accounts-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(File(storeDir, "shielded.sqlite").absolutePath)
                val walletId = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-accounts",
                    createDefaultAccounts = true,
                ).walletId
                assertEquals(
                    "a wallet never bound holds no shielded accounts",
                    emptyList<Int>(),
                    manager.shieldedOrdinaryAccounts(walletId),
                )

                manager.bindShielded(walletId, listOf(2, 1))
                assertEquals(listOf(1, 2), manager.shieldedOrdinaryAccounts(walletId))

                manager.clearShieldedStorage()
                val held = manager.shieldedOrdinaryAccounts(walletId)
                assertEquals("the accounts outlive the Clear", listOf(1, 2), held)

                manager.bindShielded(walletId, held)
                assertEquals(listOf(1, 2), manager.shieldedOrdinaryAccounts(walletId))
            }
        } finally {
            storeDir.deleteRecursively()
        }
    }

    /**
     * After a Clear's reset, [ShieldedService] registers every loaded wallet
     * other than the displayed one again, each with the accounts Rust holds
     * for it. A wallet bound with other accounts used to come back with the
     * default account 0 only (dashpay/platform#5989).
     *
     * Two things are checked for the other wallet: that it is registered
     * again with its ledger restored (Rust announces its balance state as
     * ready; the keys alone would not show that, since they outlive the
     * reset), and that the accounts of that registration are the ones it
     * had.
     *
     * The reset is run directly and then the service's fleet pass, which is
     * what a Clear runs between its native reset and its sync restart, so no
     * shielded sync loop is started. It uses this class's SDK fixture like
     * the tests around it.
     */
    @Test
    fun shieldedClearRegistersAnotherWalletAgainWithItsOwnAccounts() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "shielded-clear-accounts-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        val storePath = File(storeDir, "shielded.sqlite").absolutePath
        val service = ShieldedService(db)
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(storePath)
                val displayed = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-clear-displayed",
                    createDefaultAccounts = true,
                ).walletId
                val other = manager.createWallet(
                    mnemonic = otherTestMnemonic,
                    name = "shielded-clear-other",
                    createDefaultAccounts = true,
                ).walletId
                val otherKey = shieldedBalanceKey(other)

                service.bind(manager, displayed, storePath)
                assertTrue("the displayed wallet is bound", service.isBound.value)
                assertTrue(service.bindEngine(manager, other, storePath, listOf(2, 1)))
                assertEquals(listOf(1, 2), manager.shieldedOrdinaryAccounts(other))
                // Announced as registered first, so that each change below
                // is one Rust reports on its own.
                withTimeout(stateWaitMs) {
                    manager.shieldedBalances.first {
                        val state = it[otherKey]?.state
                        state != null && state != ShieldedLocalBalanceState.Unbound
                    }
                }

                // The reset drops the registration: Rust announces the
                // wallet as unbound, while its keys still name its accounts.
                manager.clearShieldedStorage()
                withTimeout(stateWaitMs) {
                    manager.shieldedBalances.first {
                        it[otherKey]?.state == ShieldedLocalBalanceState.Unbound
                    }
                }
                assertEquals(listOf(1, 2), manager.shieldedOrdinaryAccounts(other))

                assertTrue(service.registerOtherWalletsAgain(manager, storePath))

                // Ready, not just no longer unbound: a registration whose
                // ledger failed to restore reads as incomplete.
                val rebound = withTimeout(stateWaitMs) {
                    manager.shieldedBalances.first {
                        it[otherKey]?.state is ShieldedLocalBalanceState.Ready
                    }
                }.getValue(otherKey).state as ShieldedLocalBalanceState.Ready
                assertEquals(
                    "the coordinator registers the other wallet with the accounts it had",
                    setOf(1, 2),
                    rebound.snapshot.accounts.keys,
                )
                assertEquals(listOf(1, 2), manager.shieldedOrdinaryAccounts(other))
            }
        } finally {
            service.close()
            storeDir.deleteRecursively()
        }
    }

    /**
     * A Core to shielded "Max" (a wallet drain) across a restart, offline.
     *
     * The first manager shows the read paths on a fresh wallet: the preview
     * reports nothing to shield, and the status of an operation id nobody
     * stored is "not sent". That status is a DURABLE read, so it only
     * succeeds when the persistence bridge attests everything a drain
     * requires, the asset-lock operation ids included; without the two
     * operation-id slots Rust refuses it.
     *
     * Then a drain's asset lock is put in the store, as the persistence
     * callbacks leave it once a drain has built one, and a NEW manager is
     * created over the same database: the process restart. Rust finds the
     * lock again by its operation id, which it can only do through
     * `onLoadAssetLockOperationId`, and reports it pending with its exact
     * outpoint, the one thing a resume may spend. A second Max under a new
     * id is refused while that one is unfinished, and nothing is sent.
     *
     * Not covered here: a drain that builds and sends a lock, which needs
     * funds and a network (TEST_PLAN SH-18 to SH-20).
     */
    @Test
    fun walletDrainIsFoundByItsOperationIdAfterARestartAndASecondOneIsRefused() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "wallet-drain-round-trip-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        val storePath = File(storeDir, "shielded.sqlite").absolutePath
        val operationId = java.util.UUID.fromString("00112233-4455-6677-8899-aabbccddeeff")
        val lockOutPoint = org.dashfoundation.dashsdk.persistence.makeOutpoint(ByteArray(32) { 3 }, 0)
        try {
            val walletId: ByteArray
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(storePath)
                walletId = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "wallet-drain-round-trip",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(walletId, listOf(0))

                val preview = manager.shieldedWalletDrainPreview(walletId)
                assertEquals(WalletDrainPreview.Status.InsufficientFunds, preview.status)
                assertEquals(0L, preview.candidateInputCount)
                assertEquals(0L, preview.candidateBalanceDuffs)
                assertNull(preview.estimatedRecipientCredits)
                assertTrue(preview.poolFeeCredits > 0)

                val unknown = manager.shieldedWalletDrainStatus(walletId, operationId)
                assertEquals(WalletDrainOutcome.State.NotSent, unknown.state)
                assertNull(unknown.outpoint)
            }

            // What the callbacks store for a drain's lock: a shielded top-up
            // (5), already broadcast (1), with its operation id.
            val lockTransaction = decodableLockTransaction()
            db.assetLockDao().upsert(
                org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity(
                    outPointHex = org.dashfoundation.dashsdk.persistence.encodeOutPointHex(lockOutPoint),
                    walletId = walletId,
                    transactionBytes = lockTransaction,
                    fundingTypeRaw = 5,
                    identityIndexRaw = 0,
                    accountIndexRaw = 0,
                    amountDuffs = 100_000,
                    statusRaw = 1,
                    operationId = operationId.toWalletDrainOperationIdBytes(),
                ),
            )

            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
                reloaded.configureShielded(storePath)
                assertTrue(reloaded.loadPersistedWallets().isNotEmpty())
                reloaded.bindShielded(walletId, listOf(0))

                val found = reloaded.shieldedWalletDrainStatus(walletId, operationId)
                assertEquals(WalletDrainOutcome.State.Pending, found.state)
                assertEquals(
                    "the exact asset lock, by the id stored with it",
                    WalletDrainOutpoint(ByteArray(32) { 3 }, 0),
                    found.outpoint,
                )
                assertNull(found.error)

                // Another id still reads as not sent: the lock is found by
                // its own id, not by being the wallet's only lock.
                val otherId = java.util.UUID.fromString("ffeeddcc-bbaa-9988-7766-554433221100")
                val other = reloaded.shieldedWalletDrainStatus(walletId, otherId)
                assertEquals(WalletDrainOutcome.State.NotSent, other.state)
                assertNull(other.outpoint)

                // A second Max while the first is unfinished: refused by
                // Rust, with nothing sent under the new id.
                val recipient = reloaded.shieldedDefaultAddress(walletId)
                assertNotNull("the bound wallet has a shielded address", recipient)
                val refused = reloaded.shieldedFundFromWalletDrain(
                    walletId = walletId,
                    operationId = otherId,
                    recipientRaw43 = recipient!!,
                )
                assertEquals(WalletDrainOutcome.State.NotSent, refused.state)
                assertNull(refused.outpoint)
                assertNotNull("the refusal is returned beside the outcome", refused.error)
                assertTrue(
                    "Rust names the unfinished drain: ${refused.error?.message}",
                    refused.error!!.message.orEmpty().contains("another wallet drain is pending"),
                )
                assertEquals(
                    "and it stays not sent afterwards",
                    WalletDrainOutcome.State.NotSent,
                    reloaded.shieldedWalletDrainStatus(walletId, otherId).state,
                )
                assertEquals(
                    "the first one is untouched",
                    WalletDrainOutcome.State.Pending,
                    reloaded.shieldedWalletDrainStatus(walletId, operationId).state,
                )
            }
        } finally {
            storeDir.deleteRecursively()
        }
    }

    /**
     * The shielded recipient recorded on an asset lock across a restart,
     * offline, through the real JNI load path.
     *
     * A Core to shielded funding's lock is put in the store with the
     * recipient it was built to pay, as the persistence callbacks leave it.
     * A NEW manager over the same database restores the wallet, which hands
     * Rust that recipient only through `onLoadAssetLockShieldedRecipient`.
     * A resume naming another valid Orchard address is then refused with
     * `AssetLockRecipientMismatch`. Rust decides that before any funding
     * resolution, so nothing waits for a proof or reaches the network. A
     * trampoline that dropped the recipient would let the resume through
     * to funding instead.
     *
     * Not covered here: a resume to the recorded recipient, which needs
     * funds and a network.
     */
    @Test
    fun aResumeAfterARestartIsRefusedForAnotherRecipientThanTheLockRecorded() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "shielded-recipient-round-trip-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        val storePath = File(storeDir, "shielded.sqlite").absolutePath
        val lockTxid = ByteArray(32) { 4 }
        val lockOutPoint = org.dashfoundation.dashsdk.persistence.makeOutpoint(lockTxid, 0)
        try {
            val walletId: ByteArray
            val recorded: ByteArray
            val other: ByteArray
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(storePath)
                walletId = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-recipient-round-trip",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(walletId, listOf(0))
                // A second wallet only provides another valid Orchard address.
                val otherWalletId = manager.createWallet(
                    mnemonic = "legal winner thank year wave sausage worth useful legal winner " +
                        "thank yellow",
                    name = "another-recipient",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(otherWalletId, listOf(0))
                recorded = checkNotNull(manager.shieldedDefaultAddress(walletId)) {
                    "the bound wallet has a shielded address"
                }
                other = checkNotNull(manager.shieldedDefaultAddress(otherWalletId)) {
                    "the other bound wallet has a shielded address"
                }
            }
            assertTrue("two different recipients", !recorded.contentEquals(other))

            // A shielded top-up (5), broadcast (1), with the recipient it
            // was built to pay.
            db.assetLockDao().upsert(
                org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity(
                    outPointHex = org.dashfoundation.dashsdk.persistence.encodeOutPointHex(lockOutPoint),
                    walletId = walletId,
                    transactionBytes = decodableLockTransaction(),
                    fundingTypeRaw = 5,
                    identityIndexRaw = 0,
                    accountIndexRaw = 0,
                    amountDuffs = 100_000,
                    statusRaw = 1,
                    shieldedRecipientRaw43 = recorded,
                ),
            )

            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
                reloaded.configureShielded(storePath)
                assertTrue(reloaded.loadPersistedWallets().isNotEmpty())
                reloaded.bindShielded(walletId, listOf(0))

                val refused = runCatching {
                    reloaded.shieldedResumeFundFromAssetLock(
                        walletId = walletId,
                        outPointTxid = lockTxid,
                        outPointVout = 0,
                        recipientRaw43 = other,
                    )
                }.exceptionOrNull()
                assertTrue(
                    "a resume to another recipient is refused as a mismatch: $refused",
                    refused is org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet
                        .AssetLockRecipientMismatch,
                )
                // The exact bytes were restored, not merely some 43 bytes
                // other than the requested address: Rust names the recipient
                // it holds for the lock ("was built to pay shielded recipient
                // <hex>").
                val recordedHex = recorded.joinToString("") { "%02x".format(it) }
                assertTrue(
                    "the refusal names the recorded recipient: ${refused?.message}",
                    refused?.message.orEmpty().contains("shielded recipient $recordedHex"),
                )
            }
        } finally {
            storeDir.deleteRecursively()
        }
    }

    /**
     * An asset-lock transaction that only has to decode: version 3, type 0,
     * one input (an outpoint, an empty script, a sequence), one output (a
     * value, an empty script) and a lock time, consensus encoded.
     */
    private fun decodableLockTransaction(): ByteArray = java.io.ByteArrayOutputStream().apply {
        write(byteArrayOf(3, 0, 0, 0)) // version, type
        write(1) // input count
        write(ByteArray(32) { 0x11 }) // previous txid
        write(ByteArray(4)) // previous output index
        write(0) // script length
        write(ByteArray(4) { 0xFF.toByte() }) // sequence
        write(1) // output count
        write(ByteArray(8).also { it[0] = 0x10 }) // value
        write(0) // script length
        write(ByteArray(4)) // lock time
    }.toByteArray()

    /**
     * Credit verdicts through REAL JNI (dashpay/platform#4638), then reopen.
     *
     * The JVM `PlatformWalletPersistenceHandlerTest` restart tests hand the
     * handler its verdicts directly, so they stay green if the native side
     * drops or mis-packs one. This round goes through
     * [VerdictRoundTestHook.nativeDeliverUtxoCreditVerdictRound]: Rust
     * builds the verdicts out of a `CoreChangeSet`, `FFIPersister::store`
     * fires the verdict slot before the changeset, the JNI trampolines pack
     * the 36-byte outpoints, codes and heights and the round's
     * `utxos_added`, and the Kotlin handler stages and commits the rows. A
     * fresh manager then restores from Room, and the engine's balance must
     * count the credited coin only. Skipped when the native library was
     * built without the `test-hooks` cargo feature
     * (`./build_android.sh --test-hooks`), which is the only build with
     * that hook, unless the run passes `-PrequireTestHooks=true` (as CI
     * does): then a missing hook fails the test.
     *
     * Fixture, all paying the wallet's own BIP44 receive addresses:
     *  - tx 1 vout 0, [CREDITED]: no verdict, so it is restored.
     *  - tx 1 vout 1, [BORN_SPENT]: observed spent, so its row is born spent.
     *  - tx 1 vout 2, [DOOMED]: doomed, so its row is born spent.
     *  - tx 2 vout 0, [HELD]: stored earlier and then marked spent with no
     *    spender link (a sweep hold). Re-delivered with an uncredited
     *    verdict, it must stay spent. Without the verdict, the re-delivery
     *    would clear the row and the coin would come back.
     *
     * What a native fault does here: a dropped verdict (any of the three)
     * restores that coin. A mis-packed outpoint (txid bytes or vout
     * encoding) matches no row, so all three come back. Verdicts shifted
     * by one output put a verdict on the credited coin. A code moved onto
     * the wrong entry puts the context-free uncredited code on a fresh
     * row, which is then stored unspent. A count or length mismatch fails
     * the round, and the hook throws. Each case changes the asserted
     * restore set; [droppedVerdictsRestoreThePhantomCoins] and
     * [verdictsShiftedByOneOutputChangeTheRestoreSet] check that it does.
     * Not covered: swapping the two born-spent codes, or the observed-spent
     * height, which the Kotlin handler only checks for length.
     */
    @Test
    fun creditVerdictRoundThroughJniRestoresOnlyCreditedCoins() = runBlocking {
        val outcome = deliverVerdictScenario(
            verdicts = byteArrayOf(
                CODE_CREDITED,
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_OBSERVED_SPENT,
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_DOOMED,
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_UNCREDITED,
            ),
        )

        // Room, as the handler committed it from the JNI-delivered round.
        assertEquals(
            "every fixture output has a row",
            setOf(CREDITED, BORN_SPENT, DOOMED, HELD),
            outcome.spentByAmount.keys,
        )
        assertEquals(false, outcome.spentByAmount[CREDITED])
        assertEquals("observed-spent verdict → born spent", true, outcome.spentByAmount[BORN_SPENT])
        assertEquals("doomed verdict → born spent", true, outcome.spentByAmount[DOOMED])
        assertEquals("uncredited verdict vetoes the clear", true, outcome.spentByAmount[HELD])
        assertTrue("a verdict never mints a spender link", outcome.unlinked)

        // The restore set the load callback hands Rust, and what the engine
        // actually holds after a fresh manager loaded it.
        assertEquals(listOf(CREDITED), outcome.restoredAmounts)
        assertEquals(CREDITED, outcome.reloadedEngineBalance)
    }

    /**
     * Control for [creditVerdictRoundThroughJniRestoresOnlyCreditedCoins]:
     * the same round with its verdicts dropped restores every coin. So that
     * test's assertions do catch a lost verdict, and its fixture is not
     * passing for an unrelated reason.
     */
    @Test
    fun droppedVerdictsRestoreThePhantomCoins() = runBlocking {
        val dropped = deliverVerdictScenario(
            verdicts = byteArrayOf(CODE_CREDITED, CODE_CREDITED, CODE_CREDITED, CODE_CREDITED),
        )
        assertEquals(
            listOf(CREDITED, BORN_SPENT, DOOMED, HELD),
            dropped.restoredAmounts,
        )
        assertEquals(CREDITED + BORN_SPENT + DOOMED + HELD, dropped.reloadedEngineBalance)
    }

    /**
     * Control for [creditVerdictRoundThroughJniRestoresOnlyCreditedCoins]:
     * verdicts shifted by one output (a vout mis-pack) give a different
     * restore set.
     */
    @Test
    fun verdictsShiftedByOneOutputChangeTheRestoreSet() = runBlocking {
        val shifted = deliverVerdictScenario(
            verdicts = byteArrayOf(
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_OBSERVED_SPENT,
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_DOOMED,
                PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_UNCREDITED,
                CODE_CREDITED,
            ),
        )
        // The credited coin is born spent, the uncredited code leaves a
        // fresh row unspent, and the held coin is cleared and comes back.
        assertEquals(listOf(DOOMED, HELD), shifted.restoredAmounts)
        assertEquals(DOOMED + HELD, shifted.reloadedEngineBalance)
    }

    private class VerdictOutcome(
        val spentByAmount: Map<Long, Boolean>,
        val unlinked: Boolean,
        val restoredAmounts: List<Long>,
        val reloadedEngineBalance: Long,
    )

    /**
     * Create the wallet, store [HELD] in a verdict-free round and mark it
     * spent with no link, deliver the four-output round with [verdicts]
     * (one per output, in the fixture order of the test KDoc) through the
     * JNI hook, then read the result back from Room, from the restore
     * callback and from a freshly loaded manager.
     */
    private suspend fun deliverVerdictScenario(verdicts: ByteArray): VerdictOutcome {
        val walletId: ByteArray
        val receive: List<String>
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            walletId = manager.createWallet(
                mnemonic = testMnemonic,
                name = "verdict-round-trip",
                createDefaultAccounts = true,
            ).walletId
            val bip44 = db.accountDao().observeByWallet(walletId).first().single {
                it.accountType == 0 && it.standardTag == 0 && it.accountIndex == 0
            }
            receive = db.coreAddressDao().observeByAccount(bip44.id).first()
                .filter { it.poolTypeTag == 0 }
                .sortedBy { it.addressIndex }
                .map { it.address }
                .take(4)
            assertEquals("four BIP44 receive addresses persisted", 4, receive.size)
        }

        PlatformWalletPersistenceHandler(db, network = Network.TESTNET).use { handler ->
            // Round 1: tx 2 alone, credited, so the HELD row exists. The
            // hook is only in a library built with the `test-hooks` cargo
            // feature: without it this first call is an UnsatisfiedLinkError,
            // which skips the test, or fails it when the run asked for the
            // hook (`-PrequireTestHooks=true`). Any other error still fails it.
            try {
                VerdictRoundTestHook.nativeDeliverUtxoCreditVerdictRound(
                    handler,
                    walletId,
                    txTags = intArrayOf(2),
                    addresses = arrayOf(receive[3]),
                    amounts = longArrayOf(HELD),
                    verdicts = byteArrayOf(CODE_CREDITED),
                    spentAtHeights = intArrayOf(0),
                    blockHeight = BLOCK_HEIGHT,
                )
            } catch (e: UnsatisfiedLinkError) {
                val required = InstrumentationRegistry.getArguments()
                    .getString("requireTestHooks")
                    ?.toBoolean() == true
                if (required) {
                    throw AssertionError(
                        "requireTestHooks is set but the library has no test hook; " +
                            "build with ./build_android.sh --test-hooks",
                        e,
                    )
                }
                throw AssumptionViolatedException(
                    "library built without the test-hooks feature; run ./build_android.sh --test-hooks",
                    e,
                )
            }
            val held = db.txoDao().observeByWallet(walletId).first().single { it.amount == HELD }
            assertEquals(false, held.isSpent)
            // A sweep hold: spent, no spender link.
            assertEquals(1, db.txoDao().markSpentByOutpoint(held.outpoint, java.util.Date()))

            // Round 2: tx 1 (three outputs) plus tx 2 again, with the verdicts.
            VerdictRoundTestHook.nativeDeliverUtxoCreditVerdictRound(
                handler,
                walletId,
                txTags = intArrayOf(1, 1, 1, 2),
                addresses = arrayOf(receive[0], receive[1], receive[2], receive[3]),
                amounts = longArrayOf(CREDITED, BORN_SPENT, DOOMED, HELD),
                verdicts = verdicts,
                spentAtHeights = IntArray(verdicts.size) { i ->
                    if (verdicts[i] ==
                        PlatformWalletPersistenceHandler.UTXO_CREDIT_VERDICT_OBSERVED_SPENT
                    ) {
                        BLOCK_HEIGHT - 1
                    } else {
                        0
                    }
                },
                blockHeight = BLOCK_HEIGHT,
            )
        }

        val rows = db.txoDao().observeByWallet(walletId).first()
        val restored = PlatformWalletPersistenceHandler(db, network = Network.TESTNET).use { handler ->
            handler.onLoadWalletList()
                .single { it.walletId.contentEquals(walletId) }
                .utxos
                .map { it.valueDuffs }
                .sorted()
        }
        val balance = PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { reloaded ->
            reloaded.loadPersistedWallets()
            assertNotNull(reloaded.wallet(forWalletId = walletId))
            val balances = org.json.JSONArray(requireNotNull(reloaded.accountBalances(walletId)))
            (0 until balances.length()).sumOf { i ->
                val account = balances.getJSONObject(i)
                account.getLong("confirmed") + account.getLong("unconfirmed") +
                    account.getLong("immature") + account.getLong("locked")
            }
        }
        return VerdictOutcome(
            spentByAmount = rows.associate { it.amount to it.isSpent },
            unlinked = rows.all { it.spendingTxid == null },
            restoredAmounts = restored,
            reloadedEngineBalance = balance,
        )
    }

    private companion object {
        /** No verdict: the engine credited the output. */
        const val CODE_CREDITED: Byte = 0

        // Distinct amounts identify each fixture output in every read-back,
        // and any subset of them sums to a distinct balance.
        const val CREDITED = 100_000L
        const val BORN_SPENT = 200_000L
        const val DOOMED = 400_000L
        const val HELD = 800_000L

        const val BLOCK_HEIGHT = 1_000
    }
}
