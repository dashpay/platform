package org.dashfoundation.example.ui.contracts.moderation

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.util.Date
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.dashsdk.moderation.ContractKeptFieldValue
import org.dashfoundation.dashsdk.moderation.ContractModerationList
import org.dashfoundation.dashsdk.moderation.ContractModerationReason
import org.dashfoundation.dashsdk.moderation.ContractWarning
import org.dashfoundation.example.util.formatDate

// Shared pieces of the contract moderation screens: port of
// `ContractModerationComponents.swift` and `ModerationPagedLoader.swift`
// (SwiftExampleApp/Views/ContractModeration). Nothing here decides anything
// about moderation: what a contract keeps and refuses is the node's to say,
// and its refusal is shown word for word.

/** The list's name on screen (← `ContractModerationList.title`). */
val ContractModerationList.title: String
    get() = when (this) {
        ContractModerationList.BANLIST -> "Banlist"
        ContractModerationList.SUSPENSIONS -> "Suspensions"
        ContractModerationList.WARNINGS -> "Warnings"
    }

/** A block time in milliseconds, for display (← `moderationDate(ms:)`). */
fun moderationDate(ms: Long): String = formatDate(Date(ms))

/**
 * The message a failed read shows: the SDK's description of the refusal (a list the
 * contract does not keep, a type that keeps no records, a network below protocol version
 * 14), word for word (← `moderationErrorText`).
 */
fun moderationErrorText(error: Throwable): String = error.message ?: error.toString()

/** One page a paged moderation read answered: its rows and the next page's cursor. */
data class ModerationPage<Item, Cursor>(val items: List<Item>, val nextCursor: Cursor?)

/**
 * The rows of a paged moderation read (a list's entries, a type's removal records, the team
 * actions), loaded a page at a time by the cursor each page returns (← `ModerationPagedLoader`).
 *
 * Only the current request writes: every [reload], [begin], [clear] and [fail] starts a new
 * generation, and a completion from an older one (a "Load More" or a lookup still in flight
 * when the user refreshed) is dropped, rows, cursor, error and loading state alike. The
 * native read already running cannot be stopped, so the generation is what keeps results
 * apart. Every method runs on the main thread.
 */
class ModerationPagedLoader<Item, Cursor : Any> {
    /** Reads the page after `cursor` (null for the first); null when the SDK returned no data. */
    fun interface Fetch<Item, Cursor> {
        suspend fun page(cursor: Cursor?): ModerationPage<Item, Cursor>?
    }

    var items by mutableStateOf<List<Item>>(emptyList())
        private set
    var nextCursor by mutableStateOf<Cursor?>(null)
        private set
    var isLoading by mutableStateOf(false)
        private set
    var errorMessage by mutableStateOf<String?>(null)
        private set

    /** Whether the current source answered at least once. */
    var loadedOnce by mutableStateOf(false)
        private set

    private var generation = 0
    private var fetch: Fetch<Item, Cursor>? = null
    private var connectionIsCurrent: () -> Boolean = { true }

    /** Start over from the first page of [fetch], dropping what is shown and any older answer. */
    suspend fun reload(fetch: Fetch<Item, Cursor>, connectionIsCurrent: () -> Boolean = { true }) {
        generation++
        this.fetch = fetch
        this.connectionIsCurrent = connectionIsCurrent
        items = emptyList()
        nextCursor = null
        errorMessage = null
        loadedOnce = false
        load(fetch, cursor = null, generation = generation)
    }

    /** Load the page after the last one shown, unless a request runs or the last page was the last. */
    suspend fun loadMore() {
        val fetch = fetch ?: return
        val cursor = nextCursor ?: return
        if (isLoading || !connectionIsCurrent()) return
        load(fetch, cursor, generation)
    }

    /**
     * A new read is about to start: drop the rows, cursor and error shown and whatever a
     * running request brings back, and show loading at once, before the read is queued.
     */
    fun begin() {
        generation++
        fetch = null
        items = emptyList()
        nextCursor = null
        errorMessage = null
        loadedOnce = false
        isLoading = true
    }

    /** Show nothing, as before the first read, and drop whatever a running request brings back. */
    fun clear() {
        generation++
        fetch = null
        items = emptyList()
        nextCursor = null
        isLoading = false
        errorMessage = null
        loadedOnce = false
    }

    /** Show [message] instead of any rows, ending whatever is in flight. */
    fun fail(message: String) {
        generation++
        fetch = null
        items = emptyList()
        nextCursor = null
        isLoading = false
        errorMessage = message
        loadedOnce = true
    }

    private suspend fun load(fetch: Fetch<Item, Cursor>, cursor: Cursor?, generation: Int) {
        isLoading = true
        errorMessage = null
        try {
            val page = fetch.page(cursor)
            if (generation != this.generation || !connectionIsCurrent()) return
            items = items + page?.items.orEmpty()
            nextCursor = page?.nextCursor
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (generation != this.generation || !connectionIsCurrent()) return
            errorMessage = moderationErrorText(e)
        }
        isLoading = false
        loadedOnce = true
    }
}

/** The state of one read: what the node answered, or why it did not (← `ModerationLoad`). */
sealed class ModerationLoad<out T> {
    data object Idle : ModerationLoad<Nothing>()
    data object Loading : ModerationLoad<Nothing>()
    data class Loaded<T>(val value: T) : ModerationLoad<T>()
    data class Failed(val message: String) : ModerationLoad<Nothing>()
}

/**
 * The answer of a single moderation read (the fee pots, the action counts, a team action's
 * signers) (← `ModerationSingleLoader`). Every [reload] starts a new generation and only the
 * current one writes, so an older read finishing late cannot replace a newer answer.
 */
class ModerationSingleLoader<T> {
    var load by mutableStateOf<ModerationLoad<T>>(ModerationLoad.Idle)
        private set

    private var generation = 0

    /** Read again with [fetch], dropping whatever an older read brings back. */
    suspend fun reload(fetch: suspend () -> T, connectionIsCurrent: () -> Boolean = { true }) {
        val current = ++generation
        load = ModerationLoad.Loading
        try {
            val value = fetch()
            if (current != generation || !connectionIsCurrent()) return
            load = ModerationLoad.Loaded(value)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            if (current != generation || !connectionIsCurrent()) return
            load = ModerationLoad.Failed(moderationErrorText(e))
        }
    }

    /** Show [message], ending whatever is in flight. */
    fun fail(message: String) {
        generation++
        load = ModerationLoad.Failed(message)
    }

    val isLoading: Boolean get() = load is ModerationLoad.Loading
}

/** The kind of a [ModerationMessageRow]. */
enum class ModerationMessageKind { LOADING, ERROR, EMPTY }

/** A status line: a spinner while loading, otherwise a message (← `ModerationMessageRow`). */
@Composable
fun ModerationMessageRow(kind: ModerationMessageKind, text: String, testTag: String) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp)
            .semantics(mergeDescendants = true) {}
            .testTag(testTag),
    ) {
        when (kind) {
            ModerationMessageKind.LOADING -> {
                CircularProgressIndicator(modifier = Modifier.size(16.dp), strokeWidth = 2.dp)
                Text(
                    text,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            ModerationMessageKind.ERROR -> {
                Icon(
                    Icons.Default.Warning,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.error,
                    modifier = Modifier.size(16.dp),
                )
                SelectionContainer {
                    Text(
                        text,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
            ModerationMessageKind.EMPTY -> Text(
                text,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** A labelled value: an id, a time, a number (← `ModerationFieldRow`). */
@Composable
fun ModerationFieldRow(
    label: String,
    value: String,
    monospaced: Boolean = false,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 3.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(
            label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        SelectionContainer(modifier = Modifier.weight(1f)) {
            Text(
                value,
                style = MaterialTheme.typography.bodySmall.let {
                    if (monospaced) it.copy(fontFamily = FontFamily.Monospace) else it
                },
                maxLines = 1,
                overflow = TextOverflow.MiddleEllipsis,
                textAlign = TextAlign.End,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}

/** A "Load More" button that passes the last page's cursor. */
@Composable
fun ModerationLoadMoreButton(isLoading: Boolean, testTag: String, onClick: () -> Unit) {
    OutlinedButton(
        onClick = onClick,
        enabled = !isLoading,
        modifier = Modifier
            .fillMaxWidth()
            .testTag(testTag),
    ) {
        if (isLoading) {
            CircularProgressIndicator(
                modifier = Modifier
                    .size(14.dp)
                    .padding(end = 4.dp),
                strokeWidth = 2.dp,
            )
        }
        Text("Load More")
    }
}

/**
 * A moderation reason as the moderator wrote it (← `ModerationReasonView`). The free text
 * sits in a labelled box so it cannot pass for the structured fields below it, and its bidi
 * controls are shown as escapes.
 */
@Composable
fun ModerationReasonView(reason: ContractModerationReason) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Surface(
            shape = MaterialTheme.shapes.small,
            border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
            color = MaterialTheme.colorScheme.surface,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(Modifier.padding(6.dp)) {
                Text(
                    "Moderator's text",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                SelectionContainer {
                    Text(
                        if (reason.text.isEmpty()) "(empty)" else BidiControls.visible(reason.text),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        }
        reason.code?.let {
            Text(
                "Code $it",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        reason.documents.orEmpty().forEach { document ->
            // The type name is the moderator's text: its bidi controls shown as escapes, and
            // the id set apart so the name cannot reorder it
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                val style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace)
                Text(
                    BidiControls.visible(document.documentTypeName),
                    style = style,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    document.documentId,
                    style = style,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.MiddleEllipsis,
                )
            }
        }
        reason.reasonDocumentId?.let {
            Text(
                "Reason document $it",
                style = MaterialTheme.typography.labelSmall.copy(fontFamily = FontFamily.Monospace),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                overflow = TextOverflow.MiddleEllipsis,
            )
        }
    }
}

/** A list of warnings, oldest first (← `ModerationWarningsView`). */
@Composable
fun ModerationWarningsView(warnings: List<ContractWarning>, testTagPrefix: String) {
    warnings.forEachIndexed { index, warning ->
        Column(
            verticalArrangement = Arrangement.spacedBy(2.dp),
            modifier = Modifier
                .padding(vertical = 2.dp)
                .semantics(mergeDescendants = true) {}
                .testTag("$testTagPrefix.warning.$index"),
        ) {
            Text(
                "Warned ${moderationDate(warning.warnedAtMs)}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            ModerationReasonView(warning.reason)
        }
    }
}

/**
 * The value as one line of JSON-like text (← `ContractKeptFieldValue.displayText`): strings
 * and member names quoted and escaped as JSON escapes them, so text a document held cannot
 * read as another field. Members in name order.
 */
val ContractKeptFieldValue.displayText: String
    get() = when (this) {
        ContractKeptFieldValue.Null -> "null"
        is ContractKeptFieldValue.Bool -> value.toString()
        is ContractKeptFieldValue.Int -> value.toString()
        is ContractKeptFieldValue.UInt -> value.toString()
        is ContractKeptFieldValue.Int128 -> value
        is ContractKeptFieldValue.UInt128 -> value
        is ContractKeptFieldValue.Double -> value.toString()
        is ContractKeptFieldValue.Text -> quotedForDisplay(value)
        is ContractKeptFieldValue.Identifier -> quotedForDisplay(value)
        is ContractKeptFieldValue.Bytes -> "bytes($value)"
        is ContractKeptFieldValue.Array -> values.joinToString(", ", "[", "]") { it.displayText }
        is ContractKeptFieldValue.Object -> members.keys.sorted().joinToString(", ", "{", "}") { key ->
            "${quotedForDisplay(key)}: ${members.getValue(key).displayText}"
        }
    }

/**
 * [text] as a JSON string literal, with the Unicode bidirectional formatting controls written
 * as visible escapes, so text a document held cannot reorder what is shown around it.
 */
fun quotedForDisplay(text: String): String = BidiControls.visible(JsonPrimitive(text).toString())

/**
 * The Unicode bidirectional formatting controls (the Arabic letter mark, the left-to-right and
 * right-to-left marks, the embeddings and overrides, and the isolates), made visible where text
 * a document or a moderator wrote is shown (← `BidiControls`). Only the display changes.
 */
object BidiControls {
    val codePoints: Set<Int> = setOf(
        0x061C, 0x200E, 0x200F,
        0x202A, 0x202B, 0x202C, 0x202D, 0x202E,
        0x2066, 0x2067, 0x2068, 0x2069,
    )

    /** [text] with each control written as a visible `\u{202E}` escape. */
    fun visible(text: String): String {
        val shown = StringBuilder(text.length)
        var index = 0
        while (index < text.length) {
            val codePoint = text.codePointAt(index)
            if (codePoint in codePoints) {
                shown.append("\\u{").append(String.format("%04X", codePoint)).append('}')
            } else {
                shown.appendCodePoint(codePoint)
            }
            index += Character.charCount(codePoint)
        }
        return shown.toString()
    }
}
