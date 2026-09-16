package org.dashfoundation.example.ui.wallet

import androidx.lifecycle.ViewModelStore
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the fix for the "configuration changes discard the only recovery
 * phrase copy" review finding: [CreateWalletViewModel] must retain
 * [CreateWalletViewModel.unrecoverablePhrase] across whatever survives a
 * ViewModel's lifetime (unlike the `remember` state it replaced, which a
 * config-change-driven recreation would reset to null), and must still
 * scrub it on explicit acknowledgement and on [CreateWalletViewModel.onCleared].
 */
class CreateWalletViewModelTest {

    @Test
    fun recordUnrecoverablePhraseSetsTheField() {
        val viewModel = CreateWalletViewModel()
        assertNull(viewModel.unrecoverablePhrase)

        viewModel.recordUnrecoverablePhrase("apple banana cherry")

        assertEquals("apple banana cherry", viewModel.unrecoverablePhrase)
    }

    @Test
    fun clearUnrecoverablePhraseScrubsTheField() {
        val viewModel = CreateWalletViewModel()
        viewModel.recordUnrecoverablePhrase("apple banana cherry")

        viewModel.clearUnrecoverablePhrase()

        assertNull(viewModel.unrecoverablePhrase)
    }

    @Test
    fun onClearedScrubsAnyPhraseStillHeld() {
        // A real config-change survives ViewModel retention; a genuine
        // teardown (e.g. the screen navigated away from) tears the
        // ViewModelStore down and fires onCleared() — verified via the
        // real ViewModelStore machinery, not a direct protected-method call.
        val store = ViewModelStore()
        val viewModel = CreateWalletViewModel()
        store.put("createWallet", viewModel)
        viewModel.recordUnrecoverablePhrase("apple banana cherry")

        store.clear()

        assertNull(viewModel.unrecoverablePhrase)
    }
}
