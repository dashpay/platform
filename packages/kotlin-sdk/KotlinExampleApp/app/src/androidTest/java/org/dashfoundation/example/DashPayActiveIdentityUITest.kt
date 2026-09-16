package org.dashfoundation.example

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.util.Date
import java.util.UUID
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.ui.dashpay.DashPayActiveIdentityPreference
import org.dashfoundation.example.ui.dashpay.DashPayActiveIdentityPicker
import org.dashfoundation.example.ui.dashpay.DashPayActiveIdentityRestorationState
import org.dashfoundation.example.ui.dashpay.DashPayActiveIdentitySelection
import org.dashfoundation.example.ui.dashpay.DashPayActiveIdentityStore
import org.dashfoundation.example.ui.dashpay.dashPayRestorationScreenState
import org.dashfoundation.example.ui.dashpay.rememberDashPayActiveIdentitySelection
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DashPayActiveIdentityUITest {

    @get:Rule
    val composeRule = createComposeRule()

    @Test
    fun targetNetworkFailureExposesRetryWhenOldManagerRemainsActive() {
        val failure = IllegalStateException("target manager failed")

        val screenState = dashPayRestorationScreenState(
            network = Network.TESTNET,
            managerMatchesNetwork = false,
            restorationState = DashPayActiveIdentityRestorationState.Failed(
                network = Network.TESTNET,
                error = failure,
            ),
        )

        assertTrue(screenState is DashPayActiveIdentityRestorationState.Failed)
        assertSame(
            failure,
            (screenState as DashPayActiveIdentityRestorationState.Failed).error,
        )
    }

    @Test
    fun networkAndRetryChangesResetSelectionUntilTargetPreferenceEmits() {
        val identity = identity(seed = 1, name = "first", createdAtMillis = 1_000)
        val initialPreferences = MutableSharedFlow<DashPayActiveIdentityPreference>()
        val targetPreferences = MutableSharedFlow<DashPayActiveIdentityPreference>()
        val retryPreferences = MutableSharedFlow<DashPayActiveIdentityPreference>()
        var network by mutableStateOf(Network.TESTNET)
        var retryKey by mutableIntStateOf(0)

        composeRule.setContent {
            val preferenceFlow = remember(network, retryKey) {
                when {
                    network == Network.TESTNET -> initialPreferences
                    retryKey == 0 -> targetPreferences
                    else -> retryPreferences
                }
            }
            val selection = rememberDashPayActiveIdentitySelection(
                network = network,
                retryKey = retryKey,
                preferenceFlow = preferenceFlow,
                eligible = listOf(identity),
            )

            MaterialTheme {
                Text(
                    when (selection) {
                        DashPayActiveIdentitySelection.Loading -> "selection-loading"
                        is DashPayActiveIdentitySelection.Failed -> "selection-failed"
                        is DashPayActiveIdentitySelection.Ready ->
                            selection.activeIdentity?.mainDpnsName ?: "selection-empty"
                    },
                )
            }
        }

        emitWhenObserved(
            initialPreferences,
            DashPayActiveIdentityPreference.Ready(identityIdBase58 = null),
        )
        composeRule.onNodeWithText("first").assertIsDisplayed()

        composeRule.runOnIdle { network = Network.MAINNET }
        composeRule.onNodeWithText("selection-loading").assertIsDisplayed()

        emitWhenObserved(
            targetPreferences,
            DashPayActiveIdentityPreference.Ready(identityIdBase58 = null),
        )
        composeRule.onNodeWithText("first").assertIsDisplayed()

        composeRule.runOnIdle { retryKey++ }
        composeRule.onNodeWithText("selection-loading").assertIsDisplayed()

        emitWhenObserved(
            retryPreferences,
            DashPayActiveIdentityPreference.Ready(identityIdBase58 = null),
        )
        composeRule.onNodeWithText("first").assertIsDisplayed()
    }

    @Test
    fun reconstructedPreferenceSelectsTheVisibleIdentity() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val file = File(
            context.cacheDir,
            "dashpay-active-identity-${UUID.randomUUID()}.preferences_pb",
        )
        val identities = listOf(
            identity(seed = 1, name = "first", createdAtMillis = 1_000),
            identity(seed = 2, name = "second", createdAtMillis = 2_000),
        )

        val firstJob = SupervisorJob()
        val firstStore = store(file, firstJob)
        runBlocking {
            firstStore.select(
                Network.TESTNET,
                org.dashfoundation.example.util.Base58.encode(identities[1].identityId),
            )
            firstJob.cancelAndJoin()
        }

        val restoredJob = SupervisorJob()
        val restoredStore = store(file, restoredJob)
        try {
            composeRule.setContent {
                MaterialTheme {
                    when (
                        val selection = rememberDashPayActiveIdentitySelection(
                            network = Network.TESTNET,
                            store = restoredStore,
                            eligible = identities,
                        )
                    ) {
                        is DashPayActiveIdentitySelection.Ready -> {
                            selection.activeIdentity?.let { activeIdentity ->
                                DashPayActiveIdentityPicker(
                                    eligible = identities,
                                    selected = activeIdentity,
                                    enabled = true,
                                    onSelected = {},
                                )
                            }
                        }
                        else -> Unit
                    }
                }
            }

            composeRule.onNodeWithText("second", useUnmergedTree = true).assertIsDisplayed()
        } finally {
            runBlocking { restoredJob.cancelAndJoin() }
            file.delete()
        }
    }

    private fun store(
        file: File,
        job: Job,
    ): DashPayActiveIdentityStore {
        val dataStore: DataStore<Preferences> = PreferenceDataStoreFactory.create(
            scope = CoroutineScope(job + Dispatchers.IO),
            produceFile = { file },
        )
        return DashPayActiveIdentityStore(dataStore)
    }

    private fun emitWhenObserved(
        flow: MutableSharedFlow<DashPayActiveIdentityPreference>,
        preference: DashPayActiveIdentityPreference,
    ) {
        runBlocking {
            withTimeout(5_000) {
                flow.subscriptionCount.first { subscriberCount -> subscriberCount > 0 }
            }
            flow.emit(preference)
        }
    }

    private fun identity(
        seed: Int,
        name: String,
        createdAtMillis: Long,
    ): IdentityEntity =
        IdentityEntity(
            identityId = ByteArray(32) { seed.toByte() },
            mainDpnsName = name,
            createdAt = Date(createdAtMillis),
            networkRaw = Network.TESTNET.ffiValue,
            walletId = ByteArray(32) { 0x11 },
        )
}
