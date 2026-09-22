package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Test
import java.util.concurrent.Callable
import java.util.concurrent.CyclicBarrier
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference

/**
 * Pins the first-use provisioning boundary the AES aliases share
 * ([KeystoreManager.provisionAliasOnce], used by `secretKey`): concurrent
 * first-use callers — a mnemonic write healing onto `MASTER_ALIAS_UNBOUND`
 * and a denied-read recorder provisioning the same alias as defect
 * evidence, possibly from different `WalletStorage` instances — must
 * produce exactly ONE key and all use that same key. Unserialized, both
 * observe the alias absent and both generate; the second `generateKey`
 * replaces the key the first already persisted a mnemonic under, orphaning
 * that ciphertext (dashpay/platform#4643 review). AndroidKeyStore has no
 * JVM/Robolectric provider, so the alias table is a fake here; the real
 * Keystore is exercised by the instrumented
 * `KeystoreUnboundMasterKeyProvisioningTest`.
 */
class KeystoreAliasProvisioningTest {

    private class Key

    @Test
    fun concurrentFirstUseCallersProvisionExactlyOneKeyAndAllShareIt() {
        val callers = 16
        val aliasTable = AtomicReference<Key?>(null)
        val generated = AtomicInteger()
        val lookups = AtomicInteger()
        // Every racing caller's FIRST (unlocked, hot-path) lookup must observe
        // the alias absent before any of them is allowed to generate — the
        // exact interleaving that used to double-provision. The first
        // `callers` lookups are precisely those unlocked reads: a caller's
        // second (under-lock) lookup can only happen after it has passed the
        // barrier, which requires all first lookups to have arrived.
        val allObservedAbsent = CyclicBarrier(callers)
        val lock = Any()

        val pool = Executors.newFixedThreadPool(callers)
        try {
            val results = pool.invokeAll(
                List(callers) {
                    Callable {
                        KeystoreManager.provisionAliasOnce(
                            lock = lock,
                            lookup = {
                                val present = aliasTable.get()
                                if (lookups.incrementAndGet() <= callers) {
                                    allObservedAbsent.await(10, TimeUnit.SECONDS)
                                }
                                present
                            },
                            generate = {
                                generated.incrementAndGet()
                                Key().also { aliasTable.set(it) }
                            },
                        )
                    }
                },
            ).map { it.get(10, TimeUnit.SECONDS) }

            assertEquals("exactly one key must be generated", 1, generated.get())
            val winner = aliasTable.get()
            results.forEach { assertSame("every caller must use the surviving key", winner, it) }
        } finally {
            pool.shutdownNow()
        }
    }

    @Test
    fun presentAliasIsReturnedWithoutGenerating() {
        val existing = Key()
        val generated = AtomicInteger()

        val result = KeystoreManager.provisionAliasOnce(
            lock = Any(),
            lookup = { existing },
            generate = { generated.incrementAndGet(); Key() },
        )

        assertSame(existing, result)
        assertEquals(0, generated.get())
    }

    @Test
    fun failedGenerationDoesNotPoisonTheNextAttempt() {
        // A Keystore generation failure (StrongBox/TEE fault, transient
        // Keystore2 blip) must leave the alias absent and let the next caller
        // retry, not wedge the lock or cache the failure.
        val aliasTable = AtomicReference<Key?>(null)
        val attempts = AtomicInteger()
        val lock = Any()
        val provision = {
            KeystoreManager.provisionAliasOnce(
                lock = lock,
                lookup = { aliasTable.get() },
                generate = {
                    if (attempts.incrementAndGet() == 1) error("keystore generation failed")
                    Key().also { aliasTable.set(it) }
                },
            )
        }

        assertThrows(IllegalStateException::class.java) { provision() }
        val recovered = provision()

        assertSame(aliasTable.get(), recovered)
        assertEquals(2, attempts.get())
    }
}
