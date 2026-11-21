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
import org.junit.Assert.assertNull
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

    @get:Rule
    val temporaryFolder = TemporaryFolder()

    @Test
    fun shouldRestoreTheDevnetNameAndQuorumUrlAfterARelaunch() = runTest {
        val file = File(temporaryFolder.root, "app.preferences_pb")
        val first = newSession(file)
        first.appState.setDevnetName("palmatum")
        first.appState.setDevnetQuorumUrl("http://10.0.2.2:8080")
        advanceUntilIdle()
        first.close()

        val relaunched = newSession(file)
        try {
            val prefs = relaunched.dataStore.data.first()
            assertEquals("palmatum", prefs[stringPreferencesKey("platformDevnetName")])
            assertEquals("http://10.0.2.2:8080", prefs[stringPreferencesKey("platformQuorumURL")])
            relaunched.appState.restorePreferences()
            assertEquals("palmatum", relaunched.appState.devnetName.value)
            assertEquals("http://10.0.2.2:8080", relaunched.appState.devnetQuorumUrl.value)
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

            appState.setDevnetQuorumUrl("http://10.0.2.2:8080")
            assertEquals("http://10.0.2.2:8080", appState.resolvedDevnetQuorumUrl())
        } finally {
            session.close()
        }
    }

    @Test
    fun shouldPassTheDevnetNameToSpvOnlyOnDevnet() = runTest {
        val session = newSession(File(temporaryFolder.root, "app.preferences_pb"))
        try {
            val appState = session.appState
            assertNull(appState.spvDevnetName(Network.DEVNET))
            appState.setDevnetName("palmatum")
            assertEquals("palmatum", appState.spvDevnetName(Network.DEVNET))
            assertNull(appState.spvDevnetName(Network.TESTNET))
        } finally {
            session.close()
        }
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
        return Session(AppState(dataStore, scope), dataStore, job)
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
