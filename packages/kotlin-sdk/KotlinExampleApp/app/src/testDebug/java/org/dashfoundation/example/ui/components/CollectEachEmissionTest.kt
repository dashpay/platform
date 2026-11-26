package org.dashfoundation.example.ui.components

import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.MutableSharedFlow
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * An identity screen that is already open shows the balance a refresh wrote
 * to Room. The rewritten row equals the shown one ([IdentityEntity] compares
 * by identity id), which plain `collectAsStateWithLifecycle` drops.
 */
@RunWith(RobolectricTestRunner::class)
class CollectEachEmissionTest {

    @get:Rule
    val compose = createComposeRule()

    private val stored = IdentityEntity(identityId = ByteArray(32) { 4 }, networkRaw = 0, balance = 56_922_276_355)

    /** Room emits on every write without de-duplication, as this replaying flow does. */
    private val row = MutableSharedFlow<IdentityEntity?>(replay = 1).apply { tryEmit(stored) }

    @Test
    fun `should show the balance a refresh wrote while the screen is open`() {
        compose.setContent {
            val identity by row.collectEachAsStateWithLifecycle(initialValue = null)
            Text("${identity?.balance}", Modifier.testTag("balance"))
        }
        compose.onNodeWithTag("balance").assertTextEquals("56922276355")

        row.tryEmit(stored.copy(balance = 56_519_213_215))
        compose.waitForIdle()

        compose.onNodeWithTag("balance").assertTextEquals("56519213215")
    }

    @Test
    fun `should show why plain collection is not enough`() {
        compose.setContent {
            val identity by row.collectAsStateWithLifecycle(initialValue = null)
            Text("${identity?.balance}", Modifier.testTag("balance"))
        }
        row.tryEmit(stored.copy(balance = 56_519_213_215))
        compose.waitForIdle()

        // The rewritten row is equal by id, so the old balance stays.
        compose.onNodeWithTag("balance").assertTextEquals("56922276355")
    }
}
