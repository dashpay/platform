package org.dashfoundation.example.ui.contracts.moderation

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.moderation.ContractKeptFieldValue
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The pure parts of the contract moderation screens: how kept values and moderator text are
 * shown, how typed ids are read, and that only the current read of a loader writes (the
 * Kotlin side of `ContractModerationComponents.swift` and `ModerationPagedLoader.swift`).
 */
class ModerationSupportTest {

    @Test
    fun shouldShowKeptValuesAsJsonLikeTextWithMembersInNameOrder() {
        val value = ContractKeptFieldValue.Object(
            mapOf(
                "z" to ContractKeptFieldValue.UInt(7u),
                "a" to ContractKeptFieldValue.Array(
                    listOf(
                        ContractKeptFieldValue.Text("say \"hi\""),
                        ContractKeptFieldValue.Null,
                        ContractKeptFieldValue.Int(-3),
                    ),
                ),
                "b" to ContractKeptFieldValue.Bytes("AQID"),
            ),
        )
        assertEquals(
            """{"a": ["say \"hi\"", null, -3], "b": bytes(AQID), "z": 7}""",
            value.displayText,
        )
        assertEquals("-0.0", ContractKeptFieldValue.Double(-0.0).displayText)
        assertEquals(
            "340282366920938463463374607431768211455",
            ContractKeptFieldValue.UInt128("340282366920938463463374607431768211455").displayText,
        )
    }

    /** Text a document or a moderator wrote cannot reorder what is shown around it. */
    @Test
    fun shouldShowBidiControlsAsVisibleEscapes() {
        assertEquals("abc\\u{202E}def", BidiControls.visible("abc‮def"))
        assertEquals("\"x\\u{2066}\"", quotedForDisplay("x⁦"))
        assertEquals("plain", BidiControls.visible("plain"))
    }

    @Test
    fun shouldReadTypedDocumentIdsTrimmedWithoutEmptyOnes() {
        assertEquals(listOf("a", "b"), parseDocumentIds(" a , ,b,"))
        assertTrue(parseDocumentIds("  ").isEmpty())
    }

    /** A look up by ids is dropped when the ids change; reading every record goes on. */
    @Test
    fun shouldDropALookUpByIdsWhenTheTypedIdsChange() {
        val filter = RemovalsLookupFilter()
        val everyRecord = filter.begin(null)
        assertFalse(filter.draftChanged())
        assertTrue(filter.isCurrent(everyRecord))

        val byIds = filter.begin(listOf("a"))
        assertEquals(listOf("a"), filter.ids)
        assertTrue(filter.draftChanged())
        assertNull(filter.ids)
        assertFalse(filter.isCurrent(byIds))
    }

    /** A page from a superseded read is dropped, rows and error alike. */
    @Test
    fun shouldDropAPageFromASupersededRead() = runTest {
        val loader = ModerationPagedLoader<String, String>()
        val stale = CompletableDeferred<ModerationPage<String, String>?>()
        val first = launch { loader.reload({ stale.await() }) }
        testScheduler.runCurrent()
        assertTrue(loader.isLoading)

        loader.reload({ ModerationPage(listOf("fresh"), "next") })
        assertEquals(listOf("fresh"), loader.items)
        assertEquals("next", loader.nextCursor)

        stale.complete(ModerationPage(listOf("stale"), null))
        first.join()
        assertEquals(listOf("fresh"), loader.items)
        assertEquals("next", loader.nextCursor)
    }

    /** "Load More" passes the last cursor and appends; a page without one ends paging. */
    @Test
    fun shouldAppendTheNextPageByTheCursorTheLastOneGave() = runTest {
        val loader = ModerationPagedLoader<String, String>()
        val cursors = mutableListOf<String?>()
        loader.reload({ cursor ->
            cursors += cursor
            if (cursor == null) ModerationPage(listOf("a"), "a") else ModerationPage(listOf("b"), null)
        })
        loader.loadMore()
        loader.loadMore()
        assertEquals(listOf(null, "a"), cursors)
        assertEquals(listOf("a", "b"), loader.items)
        assertNull(loader.nextCursor)
    }

    /** An answer that arrives after the connection was replaced is not shown. */
    @Test
    fun shouldDropAnAnswerOnceTheConnectionIsReplaced() = runTest {
        val loader = ModerationSingleLoader<Int>()
        var current = true
        loader.reload({
            current = false
            1
        }, { current })
        assertTrue(loader.load is ModerationLoad.Loading)

        loader.reload({ 2 }, { true })
        assertEquals(ModerationLoad.Loaded(2), loader.load)
    }
}
