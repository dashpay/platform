package org.dashfoundation.example.state

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.emptyPreferences
import androidx.datastore.preferences.core.stringPreferencesKey
import java.io.File
import java.io.IOException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/**
 * The quorum mode the user picks survives a relaunch — port of
 * `AppStateQuorumModeTests.testShouldRestoreTheSavedQuorumModeOnLaunch`. No
 * SDK is built, so nothing native is touched.
 */
@OptIn(ExperimentalCoroutinesApi::class)
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

    /**
     * Picks are persisted and applied one at a time, each reading the latest
     * pick, so the last pick is the one that sticks. Here the first pick's
     * write is still running when the second pick arrives; applied out of
     * order, the saved mode, and the SDK's, would be the first pick while the
     * picker shows the second.
     */
    @Test
    fun lastQuorumModePickWinsWhenAnEarlierOneIsStillBeingApplied() = runTest {
        val preferences = ScriptedPreferences(firstWriteDelayMs = 1_000)
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler))
        val appState = AppState(preferences, scope)

        appState.setQuorumMode(AppState.QuorumMode.TRUSTED)
        runCurrent()
        appState.setQuorumMode(AppState.QuorumMode.SPV)
        advanceUntilIdle()

        assertEquals(AppState.QuorumMode.SPV, appState.quorumMode.value)
        assertEquals("spv", preferences.data.first()[stringPreferencesKey("quorumMode")])
    }

    /**
     * A pick whose preference write fails is reported, not thrown: thrown
     * from AppState's scope it would reach the app's uncaught-exception
     * handler and end the process.
     */
    @Test
    fun aQuorumModeThatCannotBeSavedIsReportedNotThrown() = runTest {
        val preferences = ScriptedPreferences(writeFailure = IOException("disk full"))
        val scope = CoroutineScope(SupervisorJob() + StandardTestDispatcher(testScheduler))
        val appState = AppState(preferences, scope)

        appState.setQuorumMode(AppState.QuorumMode.SPV)
        advanceUntilIdle()

        val error = appState.errorMessage.value
        assertTrue("expected the failed save to be reported, got $error", error?.contains("disk full") == true)
        assertEquals(AppState.QuorumMode.SPV, appState.quorumMode.value)
    }

    private fun TestScope.newSession(file: File): Session {
        val job = SupervisorJob()
        val scope = CoroutineScope(job + StandardTestDispatcher(testScheduler))
        val dataStore = PreferenceDataStoreFactory.create(scope = scope, produceFile = { file })
        return Session(AppState(dataStore, scope), dataStore, job)
    }

    /**
     * In-memory preferences whose writes fail with [writeFailure], or whose
     * first write takes [firstWriteDelayMs] of virtual time.
     */
    private class ScriptedPreferences(
        private val writeFailure: Exception? = null,
        private val firstWriteDelayMs: Long = 0,
    ) : DataStore<Preferences> {
        private val state = MutableStateFlow(emptyPreferences())
        private var writes = 0

        override val data: Flow<Preferences> = state

        override suspend fun updateData(
            transform: suspend (t: Preferences) -> Preferences,
        ): Preferences {
            writeFailure?.let { throw it }
            if (writes++ == 0) delay(firstWriteDelayMs)
            return transform(state.value).also { state.value = it }
        }
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
