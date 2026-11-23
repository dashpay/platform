package org.dashfoundation.example.ui.credits

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * A typed credit transfer recipient this device knows as a masternode's voter identity is
 * refused (port of the #6145 check in Swift `TransitionDetailView`).
 */
class TransferCreditsRecipientTest {

    private fun row(role: String?) = IdentityEntity(
        identityId = ByteArray(32) { 3 },
        identityType = "Masternode",
        networkRaw = 1,
        masternodeRoleRaw = role,
    )

    @Test
    fun shouldRefuseAVoterIdentityAsACreditRecipient() {
        assertNotNull(votingOnlyRecipientRefusal(row("voter")))
    }

    @Test
    fun shouldAcceptAnyOtherRecipient() {
        assertNull(votingOnlyRecipientRefusal(row("owner")))
        assertNull(votingOnlyRecipientRefusal(row(null)))
        assertNull(votingOnlyRecipientRefusal(null))
    }
}
