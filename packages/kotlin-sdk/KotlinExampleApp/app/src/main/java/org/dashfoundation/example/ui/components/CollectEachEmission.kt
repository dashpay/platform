package org.dashfoundation.example.ui.components

import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.remember
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

/** One emission, compared by identity, so no two emissions are equal. */
private class Emission<T>(val value: T)

/**
 * Like `collectAsStateWithLifecycle`, but every emission reaches the
 * composition, including one equal to the last.
 *
 * Compose state skips a value equal to the one it holds. The Room entities
 * (`TokenEntity`, `DataContractEntity`, `IdentityEntity`) compare equal by
 * id alone, so a rewritten row (a token paused, a contract's rules changed)
 * would otherwise never replace the copy a screen already shows.
 */
@Composable
fun <T> Flow<T>.collectEachAsStateWithLifecycle(initialValue: T): State<T> {
    val emissions = remember(this) { map { Emission(it) } }
    val initial = remember { Emission(initialValue) }
    val state = emissions.collectAsStateWithLifecycle(initialValue = initial)
    return remember(state) {
        object : State<T> {
            override val value: T get() = state.value.value
        }
    }
}
