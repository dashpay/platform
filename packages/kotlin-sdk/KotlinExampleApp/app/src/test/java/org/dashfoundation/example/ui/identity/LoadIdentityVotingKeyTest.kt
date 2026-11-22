package org.dashfoundation.example.ui.identity

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The voting key decisions of [LoadIdentityScreen] (port of `LoadIdentityView.swift`). */
class LoadIdentityVotingKeyTest {

    private val stored = "ab".repeat(20)
    private val entered = "cd".repeat(20)

    @Test
    fun shouldStoreTheEnteredKeyWhenTheRowNamesNone() {
        assertEquals(VotingKeyOutcome.StoreNew, decideVotingKeyOutcome(null, entered))
    }

    @Test
    fun shouldTreatTheKeyTheRowAlreadyNamesAsStored() {
        assertEquals(VotingKeyOutcome.AlreadyStored, decideVotingKeyOutcome(stored, stored))
        assertEquals(VotingKeyOutcome.AlreadyStored, decideVotingKeyOutcome(stored.uppercase(), stored))
    }

    @Test
    fun shouldKeepADifferentStoredKeyAndNameItInTheConflict() {
        assertEquals(VotingKeyOutcome.Conflicting(stored), decideVotingKeyOutcome(stored, entered))
    }

    @Test
    fun shouldQuoteRustsReasonWithoutDoublingItsPeriod() {
        assertEquals(
            "The voting private key was refused: not a WIF key.",
            votingKeyRefusedMessage("not a WIF key."),
        )
        assertEquals("The voting private key was refused.", votingKeyRefusedMessage(null))
        assertEquals("The voting private key was refused.", votingKeyRefusedMessage("  "))
    }

    @Test
    fun shouldOfferMasternodeKeysOnlyForMasternodeAndEvonode() {
        assertFalse(LoadIdentityType.USER.isMasternode)
        assertTrue(LoadIdentityType.MASTERNODE.isMasternode)
        assertTrue(LoadIdentityType.EVONODE.isMasternode)
        assertEquals(listOf("User", "Masternode", "Evonode"), LoadIdentityType.entries.map { it.rawValue })
    }
}
