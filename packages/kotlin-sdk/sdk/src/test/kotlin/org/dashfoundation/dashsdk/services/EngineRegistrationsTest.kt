package org.dashfoundation.dashsdk.services

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Coverage for [EngineRegistrations]: what a shielded Clear registers each
 * wallet with again once its Rust reset has dropped every registration.
 *
 * The real binds call into JNI, so the tests drive the ledger the way
 * [ShieldedService] does: `record` where a native bind went through, `reset`
 * where the Rust reset did, and `accountsToRestore` where the Clear's fleet
 * pass decides, per wallet, what to bind.
 */
class EngineRegistrationsTest {

    private fun id(byte: Int) = "%02x".format(byte).repeat(32)

    private val walletA = id(0x0a)
    private val walletB = id(0x0b)
    private val walletC = id(0x0c)
    private val walletD = id(0x0d)
    private val default = listOf(0)

    @Test
    fun shouldRegisterAWalletAgainWithTheAccountsItHadBeforeTheClear() {
        val manager = Any()
        val registrations = EngineRegistrations()
        registrations.record(manager, walletA, listOf(0))
        registrations.record(manager, walletB, listOf(2, 1))
        assertEquals(listOf(1, 2), registrations.standing(manager, walletB))

        registrations.reset(manager)

        assertNull(registrations.standing(manager, walletB))
        assertEquals(listOf(1, 2), registrations.accountsToRestore(manager, walletB, default))
        assertEquals(listOf(0), registrations.accountsToRestore(manager, walletA, default))
    }

    @Test
    fun shouldRegisterAWalletItNeverRegisteredWithTheDefault() {
        val manager = Any()
        val registrations = EngineRegistrations()
        // A reset with nothing recorded at all.
        registrations.reset(manager)
        assertEquals(default, registrations.accountsToRestore(manager, walletC, default))

        // And next to a wallet that was registered.
        registrations.record(manager, walletB, listOf(1, 2))
        registrations.reset(manager)
        assertEquals(default, registrations.accountsToRestore(manager, walletC, default))
    }

    @Test
    fun shouldLeaveAloneAWalletBoundAfterTheClearsReset() {
        val manager = Any()
        val registrations = EngineRegistrations()
        registrations.record(manager, walletA, listOf(0))
        registrations.record(manager, walletB, listOf(0))
        registrations.reset(manager)

        // B becomes the mirror and binds [1] before the fleet pass reaches
        // it. The pass must not put it back on what it had, or on [0].
        registrations.record(manager, walletB, listOf(1))

        assertNull(registrations.accountsToRestore(manager, walletB, default))
        assertEquals(listOf(1), registrations.standing(manager, walletB))
        // The next Clear restores what that bind registered.
        registrations.reset(manager)
        assertEquals(listOf(1), registrations.accountsToRestore(manager, walletB, default))
    }

    @Test
    fun shouldNotRegisterTwiceAWalletThePassAlreadyRestored() {
        val manager = Any()
        val registrations = EngineRegistrations()
        registrations.record(manager, walletB, listOf(1, 2))
        registrations.reset(manager)

        val accounts = registrations.accountsToRestore(manager, walletB, default)
        registrations.record(manager, walletB, accounts!!)

        assertNull(registrations.accountsToRestore(manager, walletB, default))
        assertEquals(listOf(1, 2), registrations.standing(manager, walletB))
    }

    @Test
    fun shouldNotLetAnotherManagerInheritEntries() {
        val testnet = Any()
        val mainnet = Any()
        val registrations = EngineRegistrations()
        registrations.record(testnet, walletB, listOf(1, 2))

        // The same wallet id on another manager was never registered there.
        assertNull(registrations.standing(mainnet, walletB))
        assertEquals(default, registrations.accountsToRestore(mainnet, walletB, default))

        // A Clear on the other manager drops nothing of this one.
        registrations.reset(mainnet)
        assertEquals(listOf(1, 2), registrations.standing(testnet, walletB))
        assertNull(registrations.accountsToRestore(testnet, walletB, default))

        // And a Clear on this one restores nothing on the other.
        registrations.reset(testnet)
        assertEquals(listOf(1, 2), registrations.accountsToRestore(testnet, walletB, default))
        assertEquals(default, registrations.accountsToRestore(mainnet, walletB, default))
    }

    /**
     * The Clear's fleet pass as [ShieldedService.clearLocalState] runs it:
     * every wallet but the mirror, each decided and bound on its own. B's
     * bind fails (its mnemonic cannot be read); the others must still come
     * back with their accounts, and B's account list must not be lost.
     */
    @Test
    fun shouldRestoreTheOtherWalletsWhenOneFails() = runTest {
        val manager = Any()
        val registrations = EngineRegistrations()
        registrations.record(manager, walletA, listOf(0))
        registrations.record(manager, walletB, listOf(1, 2))
        registrations.record(manager, walletC, listOf(3))
        registrations.reset(manager)
        // The Clear's re-bind of the mirror.
        registrations.record(manager, walletA, listOf(0))

        val attempted = mutableMapOf<String, List<Int>>()
        suspend fun fleetPass(failing: String?) = engineBindOtherWallets(
            allWalletIds = listOf(walletA, walletB, walletC, walletD),
            mirrorWalletId = walletA,
        ) { wallet ->
            val accounts = registrations.accountsToRestore(manager, wallet, default)
            if (accounts != null) {
                attempted[wallet] = accounts
                if (wallet == failing) throw IllegalStateException("bind failed")
                registrations.record(manager, wallet, accounts)
            }
        }
        fleetPass(failing = walletB)

        assertEquals(
            mapOf(walletB to listOf(1, 2), walletC to listOf(3), walletD to default),
            attempted,
        )
        assertNull(registrations.standing(manager, walletB))
        assertEquals(listOf(3), registrations.standing(manager, walletC))
        assertEquals(default, registrations.standing(manager, walletD))

        // B stays unregistered, with its accounts remembered: the next
        // Clear binds it with [1, 2] again, not with the default.
        registrations.reset(manager)
        registrations.record(manager, walletA, listOf(0))
        attempted.clear()
        fleetPass(failing = null)
        assertEquals(listOf(1, 2), attempted[walletB])
        assertEquals(listOf(1, 2), registrations.standing(manager, walletB))
    }
}
