package org.dashfoundation.example.ui.identity

import androidx.lifecycle.SavedStateHandle
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.identity.IdentityKeyLimitsUpdatePreview
import org.dashfoundation.example.ui.identity.UpdateKeyLimitsViewModel.Gate
import org.dashfoundation.example.ui.identity.UpdateKeyLimitsViewModel.InputPreview
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

/**
 * The Update Limits screen's state (port of `UpdateKeyLimitsView.swift`): it starts from a
 * proved refresh, offers the update only for a preview of exactly what the form shows, hands
 * that preview back unchanged, locks after any failed submission, and comes back locked when
 * the process died while an update was being sent.
 */
@OptIn(ExperimentalCoroutinesApi::class)
class UpdateKeyLimitsViewModelTest {

    private val dispatcher = StandardTestDispatcher()
    private val identityId = ByteArray(32) { 3 }

    @Before
    fun setMain() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun resetMain() {
        Dispatchers.resetMain()
    }

    private fun preview(addBudget: ULong? = null) = IdentityKeyLimitsUpdatePreview(
        identityId = identityId,
        keyId = 2,
        signingKeyId = 0,
        currentTotalBudget = 1_000uL,
        currentExpiresAt = null,
        newTotalBudget = addBudget?.let { 1_000uL + it },
        newExpiresAt = null,
    )

    private class FakeWallet(
        override val walletHandle: Long = 1L,
        val refreshError: Exception? = null,
        val previewError: ((ULong?) -> Exception?) = { null },
        val answer: (ULong?) -> IdentityKeyLimitsUpdatePreview,
    ) : KeyLimitsWallet {
        val calls = mutableListOf<String>()
        val submitted = mutableListOf<IdentityKeyLimitsUpdatePreview>()
        var updateResult: CompletableDeferred<Unit> = CompletableDeferred(Unit)

        override suspend fun refresh() {
            calls += "refresh"
            refreshError?.let { throw it }
        }

        override suspend fun preview(addBudget: ULong?, expiresAt: ULong?): IdentityKeyLimitsUpdatePreview {
            calls += "preview:$addBudget"
            previewError(addBudget)?.let { throw it }
            return answer(addBudget)
        }

        override suspend fun update(reviewed: IdentityKeyLimitsUpdatePreview) {
            calls += "update"
            submitted += reviewed
            updateResult.await()
        }
    }

    @Test
    fun shouldRefreshFromPlatformBeforeAskingWhetherTheKeyCanBeRaised() = runTest(dispatcher) {
        val wallet = FakeWallet(answer = ::preview)
        val state = UpdateKeyLimitsViewModel(SavedStateHandle())
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        assertEquals(listOf("refresh", "preview:null"), wallet.calls)
        assertEquals(Gate.Ready(preview()), state.gate)
        assertFalse(state.canSubmit)

        // A second load (a configuration change) does not refresh again.
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        assertEquals(2, wallet.calls.size)
    }

    @Test
    fun shouldOfferNothingWhenTheRefreshOrTheOpeningPreviewFails() = runTest(dispatcher) {
        val offline = FakeWallet(refreshError = IllegalStateException("offline"), answer = ::preview)
        val first = UpdateKeyLimitsViewModel(SavedStateHandle())
        first.load { Result.success(offline) }
        advanceUntilIdle()
        assertEquals(listOf("refresh"), offline.calls)
        assertTrue((first.gate as Gate.Refused).reason.contains("offline"))

        val refused = FakeWallet(previewError = { IllegalStateException("key 2 is disabled") }, answer = ::preview)
        val second = UpdateKeyLimitsViewModel(SavedStateHandle())
        second.load { Result.success(refused) }
        advanceUntilIdle()
        assertEquals(Gate.Refused("key 2 is disabled"), second.gate)

        val missing = UpdateKeyLimitsViewModel(SavedStateHandle())
        missing.load { Result.failure(IllegalStateException("Wallet not loaded in the wallet manager.")) }
        assertEquals(Gate.Refused("Wallet not loaded in the wallet manager."), missing.gate)
    }

    @Test
    fun shouldOfferTheUpdateOnlyForAPreviewOfWhatTheFormShows() = runTest(dispatcher) {
        val wallet = FakeWallet(answer = ::preview)
        val state = UpdateKeyLimitsViewModel(SavedStateHandle())
        state.load { Result.success(wallet) }
        advanceUntilIdle()

        state.onAddBudgetText("abc")
        assertEquals(InputPreview.InvalidAmount, state.inputPreview)

        state.onAddBudgetText("0.00000000005")
        assertEquals(InputPreview.Checking, state.inputPreview)
        assertFalse(state.canSubmit)
        advanceTimeBy(UpdateKeyLimitsViewModel.PREVIEW_DEBOUNCE_MILLIS + 1)
        assertEquals(preview(5uL), state.matchingPreview)
        assertTrue(state.canSubmit)

        // An edit makes the shown preview stale until its own preview answers.
        state.onAddBudgetText("0.00000000007")
        assertNull(state.matchingPreview)
        assertFalse(state.canSubmit)
        advanceUntilIdle()
        assertEquals(preview(7uL), state.matchingPreview)
        // Typing fast: only the last text was previewed after the pause.
        assertEquals(listOf("refresh", "preview:null", "preview:5", "preview:7"), wallet.calls)
    }

    @Test
    fun shouldShowRustsRefusalOfAChangeAndOfferNoUpdate() = runTest(dispatcher) {
        val wallet = FakeWallet(
            previewError = { if (it == 0uL) IllegalStateException("adding nothing would be refused") else null },
            answer = ::preview,
        )
        val state = UpdateKeyLimitsViewModel(SavedStateHandle())
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        state.onAddBudgetText("0")
        advanceUntilIdle()
        assertEquals(
            "adding nothing would be refused",
            (state.inputPreview as InputPreview.Refused).reason,
        )
        assertFalse(state.canSubmit)
    }

    @Test
    fun shouldSendExactlyTheReviewedPreviewOnceAndFinish() = runTest(dispatcher) {
        val wallet = FakeWallet(answer = ::preview)
        wallet.updateResult = CompletableDeferred()
        val saved = SavedStateHandle()
        val state = UpdateKeyLimitsViewModel(saved)
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        state.onAddBudgetText("0.00000000005")
        advanceUntilIdle()
        val reviewed = state.matchingPreview!!

        state.submit()
        state.submit()
        advanceUntilIdle()
        assertTrue(state.isSubmitting)
        assertFalse(state.canSubmit)
        assertEquals(1, wallet.submitted.size)
        assertSame(reviewed, wallet.submitted.single())

        // The form is frozen while it is sent.
        state.onAddBudgetText("1")
        assertEquals("0.00000000005", state.addBudgetText)

        wallet.updateResult.complete(Unit)
        advanceUntilIdle()
        assertTrue(state.finished)
        // Nothing more is offered once it landed, even before the screen closes.
        assertTrue(state.locked)
        assertFalse(state.canSubmit)
    }

    @Test
    fun shouldLockAfterAnyFailedSubmission() = runTest(dispatcher) {
        for (failure in listOf(
            DashSdkError.PlatformWallet.TransactionBroadcastUnconfirmed("broadcast unconfirmed"),
            IllegalStateException("could not store the raised key"),
        )) {
            val wallet = FakeWallet(answer = ::preview)
            wallet.updateResult = CompletableDeferred<Unit>().apply { completeExceptionally(failure) }
            val state = UpdateKeyLimitsViewModel(SavedStateHandle())
            state.load { Result.success(wallet) }
            advanceUntilIdle()
            state.onAddBudgetText("0.00000000005")
            advanceUntilIdle()
            state.submit()
            advanceUntilIdle()

            assertTrue(state.locked)
            assertFalse(state.finished)
            assertFalse(state.canSubmit)
            assertTrue(state.submitError!!.contains(failure.message!!.substringBefore(" (")))
            // Nothing is offered again from this screen.
            state.onAddBudgetText("0.00000000006")
            advanceUntilIdle()
            state.submit()
            advanceUntilIdle()
            assertEquals(1, wallet.submitted.size)
        }
    }

    @Test
    fun shouldComeBackLockedWhenTheProcessDiedDuringASubmission() = runTest(dispatcher) {
        val wallet = FakeWallet(answer = ::preview)
        wallet.updateResult = CompletableDeferred() // never answers: the process dies first
        val saved = SavedStateHandle()
        val state = UpdateKeyLimitsViewModel(saved)
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        state.onAddBudgetText("0.00000000005")
        advanceUntilIdle()
        state.submit()
        advanceUntilIdle()

        // The system kills the process; the back stack entry is restored with its saved state.
        val restored = UpdateKeyLimitsViewModel(SavedStateHandle(saved.keys().associateWith { saved.get<Any?>(it) }))
        assertTrue(restored.locked)
        assertTrue(restored.submitError!!.contains("may have landed"))
        assertEquals("0.00000000005", restored.addBudgetText)

        val reopened = FakeWallet(answer = ::preview)
        restored.load { Result.success(reopened) }
        advanceUntilIdle()
        // It still starts from a proved refresh, but offers no submission.
        assertEquals("refresh", reopened.calls.first())
        assertFalse(restored.canSubmit)
        restored.submit()
        advanceUntilIdle()
        assertTrue(reopened.submitted.isEmpty())
    }

    @Test
    fun shouldComeBackFinishedNotAsTheAppliedIncrementWhenSavedAfterSuccess() = runTest(dispatcher) {
        val wallet = FakeWallet(answer = ::preview)
        val saved = SavedStateHandle()
        val state = UpdateKeyLimitsViewModel(saved)
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        state.onAddBudgetText("0.00000000005")
        advanceUntilIdle()
        state.submit()
        advanceUntilIdle()
        assertTrue(state.finished)

        // The state is saved after the confirmation but before the screen closed.
        val restored = UpdateKeyLimitsViewModel(SavedStateHandle(saved.keys().associateWith { saved.get<Any?>(it) }))
        assertTrue(restored.finished)
        assertTrue(restored.locked)
        assertEquals("", restored.addBudgetText)
        val reopened = FakeWallet(answer = ::preview)
        restored.load { Result.success(reopened) }
        advanceUntilIdle()
        restored.onAddBudgetText("0.00000000005")
        advanceUntilIdle()
        restored.submit()
        advanceUntilIdle()
        assertFalse(restored.canSubmit)
        assertTrue(reopened.submitted.isEmpty())
    }

    @Test
    fun shouldNotOfferTheDatePickerForAnExpiryPastItsRange() = runTest(dispatcher) {
        fun opening(expiresAt: ULong) = IdentityKeyLimitsUpdatePreview(identityId, 2, 0, null, expiresAt, null, null)
        val far = UpdateKeyLimitsViewModel(SavedStateHandle())
        far.load { Result.success(FakeWallet(answer = { opening(ULong.MAX_VALUE) })) }
        advanceUntilIdle()
        assertFalse(far.expiryPickable)
        assertNull(far.pickedExpiryMillis)

        // An expiry on 9999-12-30 can still be moved later, and the picker is seeded within range.
        val edge = UpdateKeyLimitsViewModel(SavedStateHandle())
        val last = org.dashfoundation.example.util.KeyLimitsFormatting.maxPickableMillis()
        edge.load { Result.success(FakeWallet(answer = { opening(253_402_171_200_000uL) })) }
        advanceUntilIdle()
        assertTrue(edge.expiryPickable)
        assertTrue(edge.pickedExpiryMillis!! <= last)

        // An expiry at the end of the last year cannot be moved later here.
        val end = UpdateKeyLimitsViewModel(SavedStateHandle())
        end.load { Result.success(FakeWallet(answer = { opening(last.toULong()) })) }
        advanceUntilIdle()
        assertFalse(end.expiryPickable)
    }

    @Test
    fun shouldDropARestoredExpiryEditTheScreenNoLongerOffers() = runTest(dispatcher) {
        // A draft saved while the key's expiry was pickable, with Extend expiry on.
        val saved = SavedStateHandle(
            mapOf(
                "keyLimits.addBudgetText" to "0.00000000005",
                "keyLimits.extendExpiry" to true,
                "keyLimits.pickedExpiry" to 1_800_000_000_000L,
            ),
        )
        val state = UpdateKeyLimitsViewModel(saved)
        // Another client raised the expiry past the picker's range meanwhile.
        val wallet = FakeWallet(
            answer = { add ->
                IdentityKeyLimitsUpdatePreview(identityId, 2, 0, 1_000uL, ULong.MAX_VALUE, add?.let { 1_000uL + it }, null)
            },
        )
        state.load { Result.success(wallet) }
        advanceUntilIdle()

        assertFalse(state.expiryPickable)
        assertFalse(state.extendExpiry)
        assertTrue(state.restoredExpiryDropped)
        assertNull(state.inputs.expiresAt)
        // The budget-only top-up is previewed and offered.
        assertEquals(listOf("refresh", "preview:null", "preview:5"), wallet.calls)
        assertTrue(state.canSubmit)
    }

    @Test
    fun shouldResetARestoredPickPastThisDevicesLastPickableInstant() = runTest(dispatcher) {
        val last = org.dashfoundation.example.util.KeyLimitsFormatting.maxPickableMillis()
        // Saved on a device further west, where the same local day ends later.
        val saved = SavedStateHandle(
            mapOf("keyLimits.extendExpiry" to true, "keyLimits.pickedExpiry" to last + 1),
        )
        val state = UpdateKeyLimitsViewModel(saved)
        val wallet = FakeWallet(
            answer = { IdentityKeyLimitsUpdatePreview(identityId, 2, 0, null, 1_800_000_000_000uL, null, null) },
        )
        state.load { Result.success(wallet) }
        advanceUntilIdle()
        assertTrue(state.restoredExpiryReset)
        assertTrue(state.pickedExpiryMillis!! <= last)
        assertTrue(state.extendExpiry)

        // Saved and restored again before the person acts: the notice is still shown.
        val savedAgain = SavedStateHandle(saved.keys().associateWith { saved.get<Any?>(it) })
        val again = UpdateKeyLimitsViewModel(savedAgain)
        assertTrue(again.restoredExpiryReset)
        again.load { Result.success(wallet) }
        advanceUntilIdle()
        assertTrue(again.restoredExpiryReset)
        // Picking a date clears it, also for the next restore.
        again.onPickedExpiry(1_900_000_000_000L)
        assertFalse(again.restoredExpiryReset)
        assertFalse(
            UpdateKeyLimitsViewModel(SavedStateHandle(savedAgain.keys().associateWith { savedAgain.get<Any?>(it) }))
                .restoredExpiryReset,
        )

        // A restored pick within range is kept.
        val kept = UpdateKeyLimitsViewModel(
            SavedStateHandle(mapOf("keyLimits.extendExpiry" to true, "keyLimits.pickedExpiry" to 1_900_000_000_000L)),
        )
        kept.load { Result.success(wallet) }
        advanceUntilIdle()
        assertFalse(kept.restoredExpiryReset)
        assertEquals(1_900_000_000_000L, kept.pickedExpiryMillis)
    }

    @Test
    fun shouldRefuseEveryCallOnceTheWalletWasReloaded() = runTest(dispatcher) {
        val wallet = FakeWallet(walletHandle = 1L, answer = ::preview)
        val state = UpdateKeyLimitsViewModel(SavedStateHandle())
        state.load { Result.success(wallet) }
        advanceUntilIdle()

        val reloaded = FakeWallet(walletHandle = 2L, answer = ::preview)
        state.rebind { Result.success(reloaded) }
        state.onAddBudgetText("0.00000000005")
        advanceUntilIdle()
        val refusal = state.inputPreview as InputPreview.Refused
        assertTrue(refusal.reason.contains("reloaded"))
        assertTrue(reloaded.calls.isEmpty())
        assertFalse(state.canSubmit)
    }
}
