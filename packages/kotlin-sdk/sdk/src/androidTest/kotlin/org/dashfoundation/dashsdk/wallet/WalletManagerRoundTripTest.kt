package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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
     * tier (KOTLIN_MIGRATION_SPEC.md §7.4).
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
}
