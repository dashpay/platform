package org.dashfoundation.example.services.transitions

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class StateTransitionDefinitionsTest {

    @Test
    fun `document create routes to its dedicated bridged screen`() {
        val definition = StateTransitionDefinitions.byKey("documentCreate")

        assertNotNull(definition)
        assertTrue(definition!!.executable)
        assertEquals(DedicatedTransition.CREATE_DOCUMENT, definition.dedicatedRoute)
        assertEquals(listOf("contractId", "documentType"), definition.inputs.map { it.name })
    }

    @Test
    fun shouldListTheRawKeyMasternodeVoteOnlyAmongTheDebugEntries() {
        val debug = StateTransitionDefinitions.catalog(includeDebugOnly = true)
        val release = StateTransitionDefinitions.catalog(includeDebugOnly = false)

        assertTrue(debug.any { it.key == "masternodeVote" })
        assertFalse(release.any { it.key == "masternodeVote" })
        assertTrue(release.any { it.key == "dpnsUsername" })
        assertEquals(debug.size - 1, release.size)
    }
}
