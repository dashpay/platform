package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * The scoped lookups [KeystoreSigner] resolves a masternode voting address through during a
 * vote: open only while a scope is, nested and overlapping scopes in any closing order.
 */
class ScopedSigningAliasesTest {

    private val address = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val entryA = "mnvoting.${"01".repeat(32)}.$address"
    private val entryB = "mnvoting.${"02".repeat(32)}.$address"

    @Test
    fun shouldResolveOnlyWhileTheScopeIsOpen() {
        val aliases = ScopedSigningAliases()
        assertNull(aliases.resolve(address))
        val scope = aliases.open(address, entryA)
        assertEquals(entryA, aliases.resolve(address))
        assertEquals(entryA, aliases.resolve(address.uppercase()))
        scope.close()
        assertNull(aliases.resolve(address))
        // Closing twice changes nothing.
        scope.close()
        assertNull(aliases.resolve(address))
    }

    @Test
    fun shouldKeepTheOtherScopeWhenOverlappingScopesCloseInOpeningOrder() {
        val aliases = ScopedSigningAliases()
        val first = aliases.open(address, entryA)
        val second = aliases.open(address, entryB)
        assertEquals(entryB, aliases.resolve(address))
        first.close()
        assertEquals(entryB, aliases.resolve(address))
        second.close()
        assertNull(aliases.resolve(address))
    }

    @Test
    fun shouldRestoreTheOuterScopeWhenNestedScopesCloseInnerFirst() {
        val aliases = ScopedSigningAliases()
        val outer = aliases.open(address, entryA)
        val inner = aliases.open(address, entryB)
        inner.close()
        assertEquals(entryA, aliases.resolve(address))
        outer.close()
        assertNull(aliases.resolve(address))
    }

    @Test
    fun shouldKeepTwoScopesForTheSameEntryIndependent() {
        val aliases = ScopedSigningAliases()
        val one = aliases.open(address, entryA)
        val two = aliases.open(address, entryA)
        one.close()
        assertEquals(entryA, aliases.resolve(address))
        two.close()
        assertNull(aliases.resolve(address))
    }

    @Test
    fun shouldNotResolveAnotherLookup() {
        val aliases = ScopedSigningAliases()
        aliases.open(address, entryA).use {
            assertNull(aliases.resolve("00".repeat(20)))
        }
    }
}
