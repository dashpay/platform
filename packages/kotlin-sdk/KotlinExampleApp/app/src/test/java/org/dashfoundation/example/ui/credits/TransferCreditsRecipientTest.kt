package org.dashfoundation.example.ui.credits

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * A credit or token transfer recipient this device knows, on the sender's network, as a
 * masternode's voter identity is refused (port of the #6145 checks in SwiftExampleApp).
 */
class TransferCreditsRecipientTest {

    private fun row(role: String?) = IdentityEntity(
        identityId = ByteArray(32) { 3 },
        identityType = "Masternode",
        networkRaw = 1,
        masternodeRoleRaw = role,
    )

    @Test
    fun shouldRefuseAVoterIdentityAsACreditOrTokenRecipient() {
        assertNotNull(votingOnlyRecipientRefusal(row("voter"), 1, "credits"))
        assertTrue(votingOnlyRecipientRefusal(row("voter"), 1, "tokens")!!.contains("tokens sent to it"))
    }

    @Test
    fun shouldAcceptAnyOtherRecipient() {
        assertNull(votingOnlyRecipientRefusal(row("owner"), 1, "credits"))
        assertNull(votingOnlyRecipientRefusal(row(null), 1, "credits"))
        assertNull(votingOnlyRecipientRefusal(null, 1, "credits"))
        // A voter row stored for another network says nothing about this one.
        assertNull(votingOnlyRecipientRefusal(row("voter"), 0, "credits"))
    }
}
