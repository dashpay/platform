package org.dashfoundation.example.ui.dashpay

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import java.io.File
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.filterIsInstance
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.toHex
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class DashPayActiveIdentityStoreTest {

    @get:Rule
    val temporaryFolder = TemporaryFolder()

    @Test
    fun chosenIdentitySurvivesDataStoreReconstructionAndRemainsActive() = runTest {
        val file = preferenceFile("restart.preferences_pb")
        val first = newStore(file)
        val identities = testnetIdentities()
        val selectedId = Base58.encode(identities[1].identityId)

        first.store.select(Network.TESTNET, selectedId)
        first.close()

        val restored = newStore(file)
        try {
            val coordinator = DashPayActiveIdentityRestorationCoordinator(restored.store)
            coordinator.restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(
                        value = Unit,
                        loadedWalletIdsHex = identities.map { it.walletId!!.toHex() }.toSet(),
                    )
                },
                loadWalletOwnedIdentities = { identities },
            )

            val preference = restored.store.readyPreference(Network.TESTNET)
            val eligible = eligibleDashPayIdentities(
                identities,
                identities.map { it.walletId!!.toHex() }.toSet(),
            )

            assertEquals(selectedId, preference.identityIdBase58)
            assertSame(identities[1], resolveActiveDashPayIdentity(eligible, selectedId))
        } finally {
            restored.close()
        }
    }

    @Test
    fun selectionsAreIsolatedPerNetwork() = runTest {
        val session = newStore(preferenceFile("networks.preferences_pb"))
        try {
            session.store.select(Network.MAINNET, "mainnet-identity")
            session.store.select(Network.TESTNET, "testnet-identity")

            assertEquals(
                "mainnet-identity",
                session.store.readyPreference(Network.MAINNET).identityIdBase58,
            )
            assertEquals(
                "testnet-identity",
                session.store.readyPreference(Network.TESTNET).identityIdBase58,
            )
        } finally {
            session.close()
        }
    }

    @Test
    fun staleRestoredIdentityIsClearedAndFallsBack() = runTest {
        val session = newStore(preferenceFile("stale.preferences_pb"))
        val identities = testnetIdentities()
        try {
            session.store.select(Network.TESTNET, "identity-that-no-longer-exists")

            DashPayActiveIdentityRestorationCoordinator(session.store).restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(Unit, setOf(identities[0].walletId!!.toHex()))
                },
                loadWalletOwnedIdentities = { listOf(identities[0]) },
            )

            val eligible = eligibleDashPayIdentities(
                listOf(identities[0]),
                setOf(identities[0].walletId!!.toHex()),
            )
            assertNull(session.store.readyPreference(Network.TESTNET).identityIdBase58)
            assertSame(identities[0], resolveActiveDashPayIdentity(eligible, null))
        } finally {
            session.close()
        }
    }

    @Test
    fun restoredIdentityWithoutLoadedOwningWalletIsCleared() = runTest {
        val session = newStore(preferenceFile("unloaded-wallet.preferences_pb"))
        val identities = testnetIdentities()
        try {
            session.store.select(Network.TESTNET, Base58.encode(identities[1].identityId))

            DashPayActiveIdentityRestorationCoordinator(session.store).restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(Unit, setOf(identities[0].walletId!!.toHex()))
                },
                loadWalletOwnedIdentities = { identities },
            )

            assertNull(session.store.readyPreference(Network.TESTNET).identityIdBase58)
        } finally {
            session.close()
        }
    }

    @Test
    fun walletOrphanedRestoredIdentityIsCleared() = runTest {
        val session = newStore(preferenceFile("orphan.preferences_pb"))
        val identities = testnetIdentities()
        try {
            session.store.select(Network.TESTNET, Base58.encode(identities[1].identityId))

            DashPayActiveIdentityRestorationCoordinator(session.store).restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(
                        Unit,
                        identities.map { it.walletId!!.toHex() }.toSet(),
                    )
                },
                // A wallet-orphaned row is absent from observeWalletOwnedByNetwork.
                loadWalletOwnedIdentities = { listOf(identities[0]) },
            )

            assertNull(session.store.readyPreference(Network.TESTNET).identityIdBase58)
        } finally {
            session.close()
        }
    }

    @Test
    fun reconcilingOneNetworkNeverClearsAnother() = runTest {
        val session = newStore(preferenceFile("cross-network.preferences_pb"))
        val identities = testnetIdentities()
        try {
            session.store.select(Network.MAINNET, "mainnet-stays-selected")
            session.store.select(Network.TESTNET, Base58.encode(identities[1].identityId))

            DashPayActiveIdentityRestorationCoordinator(session.store).restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(Unit, setOf(identities[0].walletId!!.toHex()))
                },
                loadWalletOwnedIdentities = { listOf(identities[0]) },
            )

            assertEquals(
                "mainnet-stays-selected",
                session.store.readyPreference(Network.MAINNET).identityIdBase58,
            )
            assertNull(session.store.readyPreference(Network.TESTNET).identityIdBase58)
        } finally {
            session.close()
        }
    }

    @Test
    fun restorationWaitsForWalletLoadBeforeReconciling() = runTest {
        val session = newStore(preferenceFile("ordering.preferences_pb"))
        val identities = testnetIdentities()
        val selectedId = Base58.encode(identities[1].identityId)
        val walletLoadStarted = CompletableDeferred<Unit>()
        val releaseWalletLoad = CompletableDeferred<Unit>()
        val coordinator = DashPayActiveIdentityRestorationCoordinator(session.store)
        try {
            session.store.select(Network.TESTNET, selectedId)

            val restoration = backgroundScope.launch {
                coordinator.restore<Unit>(
                    network = Network.TESTNET,
                    loadWallets = {
                        walletLoadStarted.complete(Unit)
                        releaseWalletLoad.await()
                        DashPayWalletLoad(
                            Unit,
                            identities.map { it.walletId!!.toHex() }.toSet(),
                        )
                    },
                    loadWalletOwnedIdentities = { identities },
                )
            }

            walletLoadStarted.await()
            assertEquals(
                DashPayActiveIdentityRestorationState.Loading(Network.TESTNET),
                coordinator.state.value,
            )
            assertEquals(
                selectedId,
                session.store.readyPreference(Network.TESTNET).identityIdBase58,
            )

            releaseWalletLoad.complete(Unit)
            restoration.join()

            assertEquals(
                DashPayActiveIdentityRestorationState.Ready(Network.TESTNET),
                coordinator.state.value,
            )
            assertEquals(
                selectedId,
                session.store.readyPreference(Network.TESTNET).identityIdBase58,
            )
        } finally {
            session.close()
        }
    }

    @Test
    fun postBootstrapRestorationFailureIsRecoverable() = runTest {
        val session = newStore(preferenceFile("retry.preferences_pb"))
        val identities = testnetIdentities()
        val coordinator = DashPayActiveIdentityRestorationCoordinator(session.store)
        try {
            val failure = runCatching {
                coordinator.restore<Unit>(
                    network = Network.TESTNET,
                    loadWallets = {
                        throw IllegalStateException("wallet load failed")
                    },
                    loadWalletOwnedIdentities = { identities },
                )
            }.exceptionOrNull()

            assertEquals("wallet load failed", failure?.message)
            assertTrue(
                coordinator.state.value is DashPayActiveIdentityRestorationState.Failed,
            )

            coordinator.restore(
                network = Network.TESTNET,
                loadWallets = {
                    DashPayWalletLoad(
                        Unit,
                        identities.map { it.walletId!!.toHex() }.toSet(),
                    )
                },
                loadWalletOwnedIdentities = { identities },
            )

            assertEquals(
                DashPayActiveIdentityRestorationState.Ready(Network.TESTNET),
                coordinator.state.value,
            )
        } finally {
            session.close()
        }
    }

    @Test
    fun cancelledRestorationRetryPreservesFailureStateAndRethrowsCancellation() = runTest {
        val session = newStore(preferenceFile("cancelled-retry.preferences_pb"))
        val coordinator = DashPayActiveIdentityRestorationCoordinator(session.store)
        val initialFailure = IllegalStateException("wallet load failed")
        try {
            runCatching {
                coordinator.restore<Unit>(
                    network = Network.TESTNET,
                    loadWallets = { throw initialFailure },
                    loadWalletOwnedIdentities = { emptyList() },
                )
            }
            val failedState = coordinator.state.value
            assertTrue(failedState is DashPayActiveIdentityRestorationState.Failed)
            val cancellation = CancellationException("retry left composition")

            val retryFailure = runCatching {
                coordinator.restore<Unit>(
                    network = Network.TESTNET,
                    loadWallets = { throw cancellation },
                    loadWalletOwnedIdentities = { emptyList() },
                )
            }.exceptionOrNull()

            assertSame(cancellation, retryFailure)
            assertSame(failedState, coordinator.state.value)
        } finally {
            session.close()
        }
    }

    private fun TestScope.newStore(file: File): StoreSession {
        val job = SupervisorJob()
        val scope = CoroutineScope(job + StandardTestDispatcher(testScheduler))
        val dataStore: DataStore<Preferences> = PreferenceDataStoreFactory.create(
            scope = scope,
            produceFile = { file },
        )
        return StoreSession(DashPayActiveIdentityStore(dataStore), job)
    }

    private fun preferenceFile(name: String): File = File(temporaryFolder.root, name)

    private suspend fun DashPayActiveIdentityStore.readyPreference(
        network: Network,
    ): DashPayActiveIdentityPreference.Ready =
        observe(network).filterIsInstance<DashPayActiveIdentityPreference.Ready>().first()

    private fun testnetIdentities(): List<IdentityEntity> =
        listOf(
            identity(seed = 1, walletSeed = 11, createdAtMillis = 1_000),
            identity(seed = 2, walletSeed = 22, createdAtMillis = 2_000),
        )

    private fun identity(
        seed: Int,
        walletSeed: Int,
        createdAtMillis: Long,
    ): IdentityEntity =
        IdentityEntity(
            identityId = ByteArray(32) { seed.toByte() },
            mainDpnsName = "identity-$seed",
            createdAt = Date(createdAtMillis),
            networkRaw = Network.TESTNET.ffiValue,
            walletId = ByteArray(32) { walletSeed.toByte() },
        )

    private data class StoreSession(
        val store: DashPayActiveIdentityStore,
        val job: Job,
    ) {
        suspend fun close() {
            job.cancelAndJoin()
        }
    }
}
