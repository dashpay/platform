package org.dashfoundation.example.ui.tokens

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import kotlinx.coroutines.flow.MutableSharedFlow
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.ui.components.collectEachAsStateWithLifecycle
import org.dashfoundation.example.util.Base58
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * [TokenActionRows] follows the paused status as it changes on a screen
 * that is already showing: Platform's answer arriving after the first
 * frame, or the stored row being rewritten. Both arrive as a token that is
 * equal to the old one (entities compare by id), so rows cached on the
 * token, or a row collected into Compose state that skips equal values,
 * kept Resume locked as "Token is not paused" on a paused token.
 */
@RunWith(RobolectricTestRunner::class)
class TokenActionRowsTest {

    @get:Rule
    val compose = createComposeRule()

    private val actorId = ByteArray(32) { (it + 1).toByte() }
    private val identity = IdentityEntity(identityId = actorId, networkRaw = 0)

    /** Emergency actions (pause / resume) authorized to [actorId] alone. */
    private val emergencyRule =
        """{"${'$'}formatVersion":"0",""" +
            """"authorizedToMakeChange":{"${'$'}type":"identity","identity":"${Base58.encode(actorId)}"},""" +
            """"adminActionTakers":{"${'$'}type":"contractOwner"},""" +
            """"changingAuthorizedActionTakersToNoOneAllowed":false,""" +
            """"changingAdminActionTakersToNoOneAllowed":false,""" +
            """"selfChangingAdminActionTakersAllowed":false}"""

    private val storedUnpaused = TokenEntity(
        id = ByteArray(36) { 5 },
        contractId = ByteArray(32) { 5 },
        position = 0,
        name = "Pausable",
        baseSupply = "1000",
        isPaused = false,
        emergencyActionRules = emergencyRule,
        hasEmergencyActions = true,
    )

    @Test
    fun `should unlock Resume when the live status says paused after the first frame`() {
        var livePaused by mutableStateOf<Boolean?>(null)
        compose.setContent {
            TokenActionRows(
                token = storedUnpaused,
                livePaused = livePaused,
                identity = identity,
                contract = null,
                oncePerIdentityClaimed = false,
                onOpen = {},
            )
        }
        resumeRow().assertIsNotEnabled().assertTextContains("Token is not paused")
        pauseRow().assertIsEnabled()

        livePaused = true
        compose.waitForIdle()

        resumeRow().assertIsEnabled()
        pauseRow().assertIsNotEnabled().assertTextContains("Token is already paused")
        transferRow().assertIsNotEnabled().assertTextContains("Token is paused")
    }

    @Test
    fun `should unlock Resume when the stored row is rewritten as paused`() {
        // Room emits on every write, with no de-duplication; a replaying
        // shared flow does the same (a StateFlow would drop an equal row).
        val storedRow = MutableSharedFlow<TokenEntity?>(replay = 1)
        storedRow.tryEmit(storedUnpaused)
        compose.setContent {
            // Collected the way TokenActionPermissionsScreen collects Room.
            val token by storedRow.collectEachAsStateWithLifecycle(initialValue = null)
            token?.let {
                TokenActionRows(
                    token = it,
                    livePaused = null,
                    identity = identity,
                    contract = null,
                    oncePerIdentityClaimed = false,
                    onOpen = {},
                )
            }
        }
        resumeRow().assertIsNotEnabled()

        // The same token id, now paused: equal to the shown row by id.
        storedRow.tryEmit(storedUnpaused.copy(isPaused = true))
        compose.waitForIdle()

        resumeRow().assertIsEnabled()
        pauseRow().assertIsNotEnabled()
    }

    @Test
    fun `should lock Resume again when the live status says resumed`() {
        var livePaused by mutableStateOf<Boolean?>(true)
        compose.setContent {
            TokenActionRows(
                token = storedUnpaused.copy(isPaused = true),
                livePaused = livePaused,
                identity = identity,
                contract = null,
                oncePerIdentityClaimed = false,
                onOpen = {},
            )
        }
        resumeRow().assertIsEnabled()

        livePaused = false
        compose.waitForIdle()

        resumeRow().assertIsNotEnabled().assertTextContains("Token is not paused")
        pauseRow().assertIsEnabled()
    }

    private fun resumeRow() = compose.onNodeWithTag("tokenActions.row.resume")
    private fun pauseRow() = compose.onNodeWithTag("tokenActions.row.pause")
    private fun transferRow() = compose.onNodeWithTag("tokenActions.row.transfer")
}
