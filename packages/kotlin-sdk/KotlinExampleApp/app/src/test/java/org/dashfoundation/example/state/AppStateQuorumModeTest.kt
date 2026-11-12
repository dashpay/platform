package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.stringPreferencesKey
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/**
 * The quorum mode the user picks survives a relaunch — port of
 * `AppStateQuorumModeTests.testShouldRestoreTheSavedQuorumModeOnLaunch`. No
 * SDK is built, so nothing native is touched.
 */
class AppStateQuorumModeTest {

    @get:Rule
    val temporaryFolder = TemporaryFolder()

    /**
     * A fresh AppState over the same preferences reads the mode back, so a
     * user who chose SPV is not dropped back to Auto on restart. It is kept
     * under the iOS UserDefaults key and raw value.
     */
    @Test
    fun pickedQuorumModeSurvivesARelaunch() = runTest {
        val file = File(temporaryFolder.root, "app.preferences_pb")
        val first = newSession(file)
        first.appState.setQuorumMode(AppState.QuorumMode.SPV)
        advanceUntilIdle()
        first.close()

        val relaunched = newSession(file)
        try {
            assertEquals(
                "spv",
                relaunched.dataStore.data.first()[stringPreferencesKey("quorumMode")],
            )
            relaunched.appState.restorePreferences()
            assertEquals(AppState.QuorumMode.SPV, relaunched.appState.quorumMode.value)
        } finally {
            relaunched.close()
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
