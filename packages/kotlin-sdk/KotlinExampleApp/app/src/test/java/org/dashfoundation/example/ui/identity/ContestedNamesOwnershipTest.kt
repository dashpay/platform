package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Assert.assertTrue
import org.junit.Test

class ContestedNamesOwnershipTest {

    @Test
    fun `persisted wallet association permits sync`() {
        val walletId = ByteArray(32) { 7 }
        val identity = identity(walletId = walletId)

        val ownership = contestedNamesOwnership(identity)

        assertTrue(ownership is ContestedNamesOwnership.WalletOwned)
        assertTrue(walletId.contentEquals((ownership as ContestedNamesOwnership.WalletOwned).walletId))
    }

    @Test
    fun `identity without wallet association remains external`() {
        assertTrue(
            contestedNamesOwnership(identity(walletId = null)) ===
                ContestedNamesOwnership.External,
        )
    }

    private fun identity(walletId: ByteArray?) = IdentityEntity(
        identityId = ByteArray(32) { 9 },
        networkRaw = 1,
        walletId = walletId,
    )
}
