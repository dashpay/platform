package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/**
 * Devnet settings: the name and quorum URL survive a relaunch under the iOS
 * keys, the quorum URL defaults from the name, and a devnet the app cannot
 * reach the settings of never strands the app. No SDK is built, so nothing
 * native is touched.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class AppStateDevnetSettingsTest {

    private companion object {
        const val PALMATUM_URL = "https://quorums.palmatum.networks.dash.org"
    }

    @get:Rule
    val temporaryFolder = TemporaryFolder()

    @Test
    fun shouldRestoreTheDevnetNameAndQuorumUrlAfterARelaunch() = runTest {
        val file = File(temporaryFolder.root, "app.preferences_pb")
        val first = newSession(file)
        first.appState.setDevnetName("palmatum")
        first.appState.setDevnetQuorumUrl("https://10.0.2.2:8443")
        advanceUntilIdle()
        first.close()

        val relaunched = newSession(file)
        try {
            val prefs = relaunched.dataStore.data.first()
            assertEquals("palmatum", prefs[stringPreferencesKey("platformDevnetName")])
            assertEquals("https://10.0.2.2:8443", prefs[stringPreferencesKey("platformQuorumURL")])
            relaunched.appState.restorePreferences()
            assertEquals("palmatum", relaunched.appState.devnetName.value)
            assertEquals("https://10.0.2.2:8443", relaunched.appState.devnetQuorumUrl.value)
        } finally {
            relaunched.close()
        }
    }

    @Test
    fun shouldDeriveTheQuorumUrlFromTheDevnetNameWhenNoUrlIsSet() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            assertNull(appState.resolvedDevnetQuorumUrl())

            appState.setDevnetName(" palmatum ")
            assertEquals(
                "https://quorums.palmatum.networks.dash.org",
                appState.resolvedDevnetQuorumUrl(),
            )

            appState.setDevnetQuorumUrl("https://10.0.2.2:8443")
            assertEquals("https://10.0.2.2:8443", appState.resolvedDevnetQuorumUrl())
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldStartSpvFromTheAppliedDevnetNotTheEditedSettings() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            appState.setDevnetName("palmatum")
            assertNull(appState.spvDevnetName(Network.DEVNET))

            appState.recordAppliedDevnet(AppState.AppliedDevnet("palmatum", PALMATUM_URL))
            appState.setDevnetName("other")
            appState.setDevnetQuorumUrl("https://quorums.other.example")
            assertEquals("palmatum", appState.spvDevnetName(Network.DEVNET))
            assertNull(appState.spvDevnetName(Network.TESTNET))

            var lookedUp: String? = null
            val peers = appState.spvPeerOverride(Network.DEVNET) { url ->
                lookedUp = url
                listOf("34.1.2.3:20001")
            }
            assertEquals(PALMATUM_URL, lookedUp)
            assertEquals(listOf("34.1.2.3:20001"), peers)
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldRefuseToStartDevnetSpvWithoutPeers() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            val notApplied = failureOf { appState.spvPeerOverride(Network.DEVNET) { emptyList() } }
            assertEquals(AppState.DEVNET_NOT_APPLIED, notApplied.message)

            appState.recordAppliedDevnet(AppState.AppliedDevnet("palmatum", PALMATUM_URL))
            for (found in listOf(null, emptyList<String>())) {
                val noPeers = failureOf { appState.spvPeerOverride(Network.DEVNET) { found } }
                assertEquals(IllegalStateException::class.java, noPeers.javaClass)
            }
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldDiscoverDevnetPeersWhenTheCustomPeerListIsEmpty() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            appState.recordAppliedDevnet(AppState.AppliedDevnet("palmatum", PALMATUM_URL))
            appState.setUseLocalhostCore(true)
            appState.setLocalCorePeers(" , ")
            val discovered = listOf("34.1.2.3:20001")
            assertEquals(discovered, appState.spvPeerOverride(Network.DEVNET) { discovered })
            assertEquals(emptyList<String>(), appState.spvPeerOverride(Network.TESTNET) { discovered })

            appState.setLocalCorePeers("127.0.0.1:19999")
            assertEquals(
                listOf("127.0.0.1:19999"),
                appState.spvPeerOverride(Network.DEVNET) { discovered },
            )
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldDropPeersFoundForADevnetThatWasReplacedDuringTheLookup() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            appState.recordAppliedDevnet(AppState.AppliedDevnet("palmatum", PALMATUM_URL))
            val replaced = failureOf {
                appState.spvPeerOverride(Network.DEVNET) {
                    appState.recordAppliedDevnet(
                        AppState.AppliedDevnet("other", "https://quorums.other.example"),
                    )
                    listOf("34.1.2.3:20001")
                }
            }
            assertEquals(AppState.DEVNET_CHANGED_DURING_LOOKUP, replaced.message)
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldRefuseAReservedDevnetNameAndANonHttpsQuorumUrl() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            appState.setDevnetName("mainnet")
            assertThrows(IllegalArgumentException::class.java) { appState.resolvedDevnetQuorumUrl() }

            // A saved Devnet whose name is refused starts on the default network.
            session.dataStore.edit {
                it[stringPreferencesKey("currentNetwork")] = Network.DEVNET.networkName
            }
            advanceUntilIdle()
            appState.restorePreferences()
            assertEquals(Network.DEFAULT, appState.currentNetwork.value)

            // A set URL does not let a refused name through to SPV.
            appState.setDevnetName("mainnet")
            appState.setDevnetQuorumUrl("https://10.0.2.2:8443")
            assertThrows(IllegalArgumentException::class.java) { appState.resolvedDevnetQuorumUrl() }

            appState.setDevnetName("palmatum")
            appState.setDevnetQuorumUrl("http://10.0.2.2:8080")
            val cleartext = assertThrows(IllegalArgumentException::class.java) {
                appState.resolvedDevnetQuorumUrl()
            }
            assertEquals(AppState.DEVNET_QUORUM_URL_NOT_HTTPS, cleartext.message)
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldLeaveASavedNetworkAndKeepTheDevnetSettings() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            appState.setDevnetName("palmatum")
            session.dataStore.edit {
                it[stringPreferencesKey("currentNetwork")] = Network.DEVNET.networkName
            }
            advanceUntilIdle()
            appState.restorePreferences()
            assertEquals(Network.DEVNET, appState.currentNetwork.value)

            appState.leaveSavedNetwork()
            assertEquals(Network.DEFAULT, appState.currentNetwork.value)
            val prefs = session.dataStore.data.first()
            assertEquals(Network.DEFAULT.networkName, prefs[stringPreferencesKey("currentNetwork")])
            assertEquals("palmatum", prefs[stringPreferencesKey("platformDevnetName")])
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldNotApplyDevnetSettingsOnceTheAppLeftDevnet() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            session.appState.setDevnetName("palmatum")
            session.appState.applyDevnetSettings()
            assertNull(session.appState.sdk.value)
            assertNull(session.appState.appliedDevnet.value)
            assertFalse(session.appState.isLoading.value)
        } finally {
            session.close()
        }
    }

    /** The exception [block] throws; fails the test when it returns. */
    private suspend fun failureOf(block: suspend () -> Unit): Exception {
        try {
            block()
        } catch (e: Exception) {
            return e
        }
        fail("expected a failure")
        error("unreachable")
    }

    @Test
    fun shouldStartOnTheDefaultNetworkWhenSavedDevnetHasNoSettings() = runTest {
        val file = File(temporaryFolder.root, "app.preferences_pb")
        val session = newSession(file)
        try {
            session.dataStore.edit {
                it[stringPreferencesKey("currentNetwork")] = Network.DEVNET.networkName
            }
            session.appState.restorePreferences()
            assertEquals(Network.DEFAULT, session.appState.currentNetwork.value)

            session.appState.setDevnetName("palmatum")
            advanceUntilIdle()
            session.appState.restorePreferences()
            assertEquals(Network.DEVNET, session.appState.currentNetwork.value)
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldRefuseADevnetSdkWithoutSettingsWithAMessageThatSaysWhatToSet() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            try {
                session.appState.switchNetwork(Network.DEVNET)
                fail("expected the devnet switch to fail")
            } catch (e: IllegalStateException) {
                assertEquals(AppState.DEVNET_SETTINGS_MISSING, e.message)
            }
            assertEquals(AppState.DEVNET_SETTINGS_MISSING, session.appState.errorMessage.value)
            assertEquals(Network.DEFAULT, session.appState.currentNetwork.value)
        } finally {
            session.close()
        }
    }

    private fun TestScope.newSession(file: File): Session {
        val job = SupervisorJob()
        val scope = CoroutineScope(job + StandardTestDispatcher(testScheduler))
        val dataStore = PreferenceDataStoreFactory.create(scope = scope, produceFile = { file })
        return Session(AppState(dataStore, scope, ::fakeDevnetQuorumBaseUrl), dataStore, job)
    }

    /** Stands in for the Rust resolver: refuses `mainnet`, as it does. */
    private fun fakeDevnetQuorumBaseUrl(name: String): String {
        check(name != "mainnet") { "Devnet name 'mainnet' is reserved" }
        return "https://quorums.$name.networks.dash.org"
    }

    private class Session(
        val appState: AppState,
        val dataStore: DataStore<Preferences>,
        val job: Job,
    ) {
        suspend fun close() {
            job.cancelAndJoin()
        }
    }
}
