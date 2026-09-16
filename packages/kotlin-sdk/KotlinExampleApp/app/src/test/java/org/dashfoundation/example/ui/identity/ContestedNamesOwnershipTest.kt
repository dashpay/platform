package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Assert.assertTrue
import org.junit.Test

class ContestedNamesOwnershipTest {

    @Test
    fun `persisted wallet association permits sync even when isLocal is false`() {
        val walletId = ByteArray(32) { 7 }
        val identity = identity(isLocal = false, walletId = walletId)

        val ownership = contestedNamesOwnership(identity)

        assertTrue(ownership is ContestedNamesOwnership.WalletOwned)
        assertTrue(walletId.contentEquals((ownership as ContestedNamesOwnership.WalletOwned).walletId))
    }

    @Test
    fun `identity without wallet association remains external regardless of isLocal hint`() {
        assertTrue(
            contestedNamesOwnership(identity(isLocal = true, walletId = null)) ===
                ContestedNamesOwnership.External,
        )
        assertTrue(
            contestedNamesOwnership(identity(isLocal = false, walletId = null)) ===
                ContestedNamesOwnership.External,
        )
    }

    private fun identity(isLocal: Boolean, walletId: ByteArray?) = IdentityEntity(
        identityId = ByteArray(32) { 9 },
        isLocal = isLocal,
        networkRaw = 1,
        walletId = walletId,
    )
}
