package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.math.BigInteger
import java.security.MessageDigest

/**
 * End to end through JNI: [PlatformWalletManager.coreAddressPrivateKey]
 * reveals the key of a receive address the wallet derived, with the mnemonic
 * read through the manager's resolver, and refuses an address the wallet does
 * not track. Offline: the wallet is created from a test mnemonic and nothing
 * connects. Port of the reveal behind Swift's `CoreAddressDetailView`
 * (`StorageRecordDetailViews.swift`); the derivation itself is covered by the
 * platform-wallet tests.
 */
@RunWith(AndroidJUnit4::class)
class CoreAddressPrivateKeyRevealTest {

    // BIP39 English test vector (all-zero entropy), as in
    // WalletManagerRoundTripTest; nothing is funded or broadcast.
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    // A valid testnet P2PKH address this wallet does not track.
    private val foreignAddress = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA"

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    @Test
    fun shouldRevealTheKeyOfAnOwnAddressAndRefuseAForeignOne() = runBlocking {
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            val created = manager.createWallet(
                mnemonic = testMnemonic,
                name = "address-key-reveal",
                createDefaultAccounts = true,
            )
            val wallet = manager.wallet(forWalletId = created.walletId)
            assertNotNull("created wallet is addressable", wallet)
            val address = wallet!!.coreWallet().use { it.nextReceiveAddress() }

            val key = manager.coreAddressPrivateKey(created.walletId, address)
            try {
                val hex = String(key.hex)
                assertEquals(64, hex.length)
                assertTrue("hex is lowercase hex", hex.all { it in '0'..'9' || it in 'a'..'f' })
                // Compressed testnet WIF: 0xef ‖ secret ‖ 0x01, base58check.
                val payload = base58CheckDecode(String(key.wif))
                assertEquals(34, payload.size)
                assertEquals(0xef.toByte(), payload[0])
                assertEquals(0x01.toByte(), payload[33])
                assertArrayEquals(hex.hexToBytes(), payload.copyOfRange(1, 33))
            } finally {
                key.clear()
            }

            // Rust owns address validity: a foreign or an empty address comes
            // back as the SDK's error type, not a Kotlin precheck.
            for (refused in listOf(foreignAddress, "")) {
                val refusal = runCatching {
                    manager.coreAddressPrivateKey(created.walletId, refused)
                }.exceptionOrNull()
                assertTrue(
                    "address '$refused' is refused with a DashSdkError, got: $refusal",
                    refusal is DashSdkError,
                )
            }
            val unknownWallet = runCatching {
                manager.coreAddressPrivateKey(ByteArray(32) { 0x7f }, address)
            }.exceptionOrNull()
            assertTrue(
                "a wallet the manager has not loaded is NotFound, got: $unknownWallet",
                unknownWallet is DashSdkError.NotFound,
            )
        }
    }

    private fun String.hexToBytes(): ByteArray =
        ByteArray(length / 2) { substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    private fun base58CheckDecode(text: String): ByteArray {
        val alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
        var value = BigInteger.ZERO
        for (c in text) {
            val digit = alphabet.indexOf(c)
            require(digit >= 0) { "not base58" }
            value = value.multiply(BigInteger.valueOf(58)).add(BigInteger.valueOf(digit.toLong()))
        }
        val bytes = value.toByteArray().let { if (it[0] == 0.toByte()) it.copyOfRange(1, it.size) else it }
        val leadingZeros = text.takeWhile { it == '1' }.length
        val full = ByteArray(leadingZeros) + bytes
        val payload = full.copyOfRange(0, full.size - 4)
        val sha = MessageDigest.getInstance("SHA-256")
        val checksum = sha.digest(sha.digest(payload)).copyOfRange(0, 4)
        assertArrayEquals("base58check checksum", checksum, full.copyOfRange(full.size - 4, full.size))
        return payload
    }
}
