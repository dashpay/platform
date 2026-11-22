package org.dashfoundation.example.ui.voting

import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The pure logic behind [MasternodeVoterSection] (port of `MasternodeVoterSection.swift`). */
class MasternodeVoterSectionTest {

    private val network = 1
    private val proTxHashHex = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
    private val keyHex = "11".repeat(32)

    private fun identity(
        firstByte: Int,
        type: String = "Masternode",
        votingKey: String? = "mnvoting." + "01".repeat(32) + "." + "ab".repeat(20),
        networkRaw: Int = network,
        alias: String? = null,
        mainDpnsName: String? = null,
        dpnsName: String? = null,
    ) = IdentityEntity(
        identityId = ByteArray(32) { if (it == 0) firstByte.toByte() else it.toByte() },
        identityType = type,
        votingPrivateKeyIdentifier = votingKey,
        networkRaw = networkRaw,
        alias = alias,
        mainDpnsName = mainDpnsName,
        dpnsName = dpnsName,
    )

    @Test
    fun shouldKeepOnlyMasternodeAndEvonodeIdentitiesOnTheNetworkWithAVotingKey() {
        val masternode = identity(1, alias = "a")
        val evonode = identity(2, type = "Evonode", alias = "b")
        val user = identity(3, type = "User", alias = "c")
        val noKey = identity(4, votingKey = null, alias = "d")
        val otherNetwork = identity(5, networkRaw = network + 1, alias = "e")

        val candidates = votingCandidates(listOf(user, noKey, evonode, otherNetwork, masternode), network)

        assertEquals(listOf(masternode, evonode), candidates)
    }

    @Test
    fun shouldSortCandidatesByDisplayName() {
        val zed = identity(1, alias = "zed")
        val amy = identity(2, mainDpnsName = "amy")
        val max = identity(3, dpnsName = "max")

        assertEquals(listOf(amy, max, zed), votingCandidates(listOf(zed, max, amy), network))
    }

    @Test
    fun shouldPreferAliasThenMainDpnsNameThenDpnsNameThenTheIdStart() {
        assertEquals("alias", voterDisplayName(identity(1, alias = "alias", mainDpnsName = "main")))
        assertEquals("main", voterDisplayName(identity(1, alias = "", mainDpnsName = "main", dpnsName = "d")))
        assertEquals("d", voterDisplayName(identity(1, dpnsName = "d")))
        val unnamed = voterDisplayName(identity(1, dpnsName = ""))
        assertTrue(unnamed, unnamed.endsWith("...") && unnamed.length == 15)
    }

    @Test
    fun shouldLabelWithTheTypeAndAddTheIdStartOnlyWhenNamesCollide() {
        val first = identity(0x0a, alias = "node")
        val second = identity(0x0b, type = "Evonode", alias = "node")
        val other = identity(0x0c, alias = "other")
        val candidates = listOf(first, second, other)

        assertEquals("node (Masternode, 0a010203)", voterLabel(first, candidates))
        assertEquals("node (Evonode, 0b010203)", voterLabel(second, candidates))
        assertEquals("other (Masternode)", voterLabel(other, candidates))
    }

    @Test
    fun shouldReverseTheIdentityIdIntoWireOrder() {
        val identityId = ByteArray(32) { it.toByte() }

        val wire = wireOrderProTxHash(identityId)

        assertArrayEquals(ByteArray(32) { (31 - it).toByte() }, wire)
        assertArrayEquals(wire, MasternodeVoter.StoredKey(identityId).wireOrderProTxHash)
        assertArrayEquals(identityId, ByteArray(32) { it.toByte() }) // input untouched
    }

    @Test
    fun shouldDecodeOnlyExactly64HexCharacters() {
        assertEquals(32, decodeTyped32Bytes("  $proTxHashHex\n")?.size)
        assertEquals(32, decodeTyped32Bytes(proTxHashHex.uppercase())?.size)
        assertNull(decodeTyped32Bytes(proTxHashHex.dropLast(1)))
        assertNull(decodeTyped32Bytes(proTxHashHex + "0"))
        assertNull(decodeTyped32Bytes(proTxHashHex.dropLast(1) + "g"))
        assertNull(decodeTyped32Bytes(""))
    }

    @Test
    fun shouldKeepTheSelectionWhileItIsACandidateElseTakeTheFirst() {
        val first = identity(1, alias = "a")
        val second = identity(2, alias = "b")
        val secondHex = second.identityId.joinToString("") { "%02x".format(it) }

        assertEquals(second, effectiveVoterSelection(secondHex, listOf(first, second)))
        assertEquals(first, effectiveVoterSelection(secondHex, listOf(first)))
        assertEquals(first, effectiveVoterSelection(null, listOf(first, second)))
        assertNull(effectiveVoterSelection(secondHex, emptyList()))
    }

    @Test
    fun shouldPublishTheSelectedIdentityAsAStoredKeyVoter() {
        val selected = identity(1)

        val voter = publish(selected = selected)

        assertEquals(MasternodeVoter.StoredKey(selected.identityId), voter)
        assertNull(publish(selected = null))
    }

    @Test
    fun shouldPublishATypedProTxHashOnlyWhenTheDialogAllowsIt() {
        val selected = identity(1)

        val typed = publish(selected = selected, allowsTyped = true, useTyped = true, proTxHash = proTxHashHex)
        val refused = publish(selected = selected, allowsTyped = false, useTyped = true, proTxHash = proTxHashHex)
        val malformed = publish(selected = selected, allowsTyped = true, useTyped = true, proTxHash = "abc")

        assertTrue(typed is MasternodeVoter.TypedProTxHash)
        assertArrayEquals(decodeTyped32Bytes(proTxHashHex), typed!!.wireOrderProTxHash)
        assertEquals(MasternodeVoter.StoredKey(selected.identityId), refused)
        assertNull(malformed)
    }

    @Test
    fun shouldNeverPublishTypedCredentialsInAReleaseBuild() {
        val selected = identity(1)

        val voter = publish(
            selected = selected,
            debugBuild = false,
            useCredentials = true,
            proTxHash = proTxHashHex,
            key = keyHex,
        )

        assertEquals(MasternodeVoter.StoredKey(selected.identityId), voter)
    }

    @Test
    fun shouldPublishTypedCredentialsWithoutAKeyUntilTheKeyIsValid() {
        val withoutKey = publish(debugBuild = true, useCredentials = true, proTxHash = proTxHashHex, key = "12")
        val withKey = publish(debugBuild = true, useCredentials = true, proTxHash = proTxHashHex, key = keyHex)
        val noProTxHash = publish(debugBuild = true, useCredentials = true, proTxHash = "", key = keyHex)

        assertTrue(withoutKey is MasternodeVoter.TypedCredentials)
        assertFalse(withoutKey!!.canCast)
        assertTrue(withKey!!.canCast)
        assertArrayEquals(
            decodeTyped32Bytes(keyHex),
            (withKey as MasternodeVoter.TypedCredentials).votingPrivateKeyCopy(),
        )
        assertNull(noProTxHash)
    }

    @Test
    fun shouldCastOnlyAsAStoredKeyOrTypedCredentialsWithAKey() {
        val hash = ByteArray(32)

        assertTrue(MasternodeVoter.StoredKey(hash).canCast)
        assertFalse(MasternodeVoter.TypedProTxHash(hash).canCast)
        assertFalse(MasternodeVoter.TypedCredentials(hash, null).canCast)
        assertTrue(MasternodeVoter.TypedCredentials(hash, ByteArray(32)).canCast)
    }

    @Test
    fun shouldNeverPrintTheTypedVotingKey() {
        val key = decodeTyped32Bytes(keyHex)

        val text = MasternodeVoter.TypedCredentials(ByteArray(32), key).toString()

        assertFalse(text, text.contains(keyHex))
    }

    private fun publish(
        selected: IdentityEntity? = null,
        allowsTyped: Boolean = false,
        useTyped: Boolean = false,
        proTxHash: String = "",
        debugBuild: Boolean = false,
        useCredentials: Boolean = false,
        key: String = "",
    ): MasternodeVoter? = publishVoter(
        selected = selected,
        allowsTypedProTxHash = allowsTyped,
        useTypedProTxHash = useTyped,
        proTxHashText = proTxHash,
        debugBuild = debugBuild,
        useTypedCredentials = useCredentials,
        votingKeyText = key,
    )
}
