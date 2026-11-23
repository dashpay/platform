package org.dashfoundation.example.ui.voting

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKey
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKeyUse
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyAddress
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheck
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheckOutcome
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRules
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeySlot
import org.dashfoundation.dashsdk.voting.recordedVotingKey
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment

/**
 * The voting key check of [MasternodeVoterSection] (port of the #6086 part of
 * `MasternodeVoterSection.swift`): which candidates may vote, which is picked by default, the
 * status line, and the check when a candidate is picked, against an in-memory database. The
 * rules are a stand-in that returns what each test sets; Rust's are tested in rs-sdk.
 */
@RunWith(RobolectricTestRunner::class)
class MasternodeVoterKeyCheckTest {

    private val testnet = 1
    private val identityId = ByteArray(32) { (it + 1).toByte() }
    private val addressA = MasternodeVotingKeyAddress.of(ByteArray(20) { 0x0a })!!
    private val addressB = MasternodeVotingKeyAddress.of(ByteArray(20) { 0x0b })!!
    private val identifierA = MasternodeVotingKeyStore.identifierFor(identityId, addressA.bytes)
    private val identifierB = MasternodeVotingKeyStore.identifierFor(identityId, addressB.bytes)

    private lateinit var db: DashDatabase
    private lateinit var rules: StubRules

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(RuntimeEnvironment.getApplication())
        rules = StubRules()
    }

    @After
    fun tearDown() {
        db.close()
    }

    // ── Candidates ───────────────────────────────────────────────────────

    @Test
    fun shouldAskRustWithTheStoredKeysAddressFromItsIdentifier() {
        val recorded = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val state = voterKeyState(row(identifierA, recorded))
        rules.use = MasternodeRecordedVotingKeyUse(canVote = false, checkBeforeUse = false)

        assertTrue(isVotingKeyRefused(state, rules))
        assertEquals(listOf(recorded to addressA), rules.usedWith)
        // No stored key, no question.
        assertFalse(isVotingKeyRefused(voterKeyState(row(null)), rules))
        assertFalse(isVotingKeyRefused(null, rules))
        assertEquals(1, rules.usedWith.size)
    }

    @Test
    fun shouldLabelARefusedCandidate() {
        val identity = row(identifierA).copy(alias = "mn")
        assertEquals("mn (Masternode, voting key does not match)", voterLabel(identity, listOf(identity), refused = true))
        assertEquals("mn (Masternode)", voterLabel(identity, listOf(identity)))
    }

    /** Swift `selectDefaultIfNeeded`: prefer an identity whose key may vote; a pick is kept. */
    @Test
    fun shouldPickACandidateThatMayVoteByDefault() {
        val refused = row(identifierA, id = 1)
        val allowed = row(identifierA, id = 2)
        val candidates = listOf(refused, allowed)
        val isRefused = { it: IdentityEntity -> it === refused }

        assertEquals(allowed, effectiveVoterSelection(null, candidates, isRefused))
        assertEquals(refused, effectiveVoterSelection(hex(refused), candidates, isRefused))
        assertEquals(refused, effectiveVoterSelection(null, listOf(refused), isRefused))
    }

    @Test
    fun shouldNotPublishARefusedVoter() {
        val identity = row(identifierA)
        fun publish(refused: Boolean) = publishVoter(
            selected = identity,
            selectedRefused = refused,
            allowsTypedProTxHash = false,
            useTypedProTxHash = false,
            proTxHashText = "",
            debugBuild = false,
            useTypedCredentials = false,
            votingKeyText = "",
        )
        assertNull(publish(refused = true))
        assertEquals(MasternodeVoter.StoredKey(identityId, identityId.reversedArray()), publish(refused = false))
    }

    // ── Status line ──────────────────────────────────────────────────────

    /** The line follows Rust's answer for the stored key, not the record alone. */
    @Test
    fun shouldShowARefusalOrAnUncheckedKeyOnlyAsRustReadsIt() {
        fun state(check: MasternodeVotingKeyCheck) =
            VoterKeyState(identifierA, addressA, MasternodeRecordedVotingKey(check, addressA))
        val refusedUse = MasternodeRecordedVotingKeyUse(canVote = false, checkBeforeUse = false)
        val recheckUse = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = true)
        val provenUse = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = false)

        assertEquals("Checking the voting key..." to false, votingKeyCheckStatusText(null, null, true, null))
        val refused = votingKeyCheckStatusText(state(MasternodeVotingKeyCheck.Invalid("Rotated.")), refusedUse, false, null)
        assertEquals(true, refused?.second)
        assertTrue(refused!!.first, refused.first.startsWith("This voting key cannot vote for this masternode: Rotated."))
        assertEquals(
            "Down. You can still vote with it; it is checked again when you pick this identity." to false,
            votingKeyCheckStatusText(state(MasternodeVotingKeyCheck.Undetermined("Down.")), recheckUse, false, null),
        )
        assertEquals(
            "Not checked. You can still vote with it." to false,
            votingKeyCheckStatusText(state(MasternodeVotingKeyCheck.Valid), provenUse, false, "Not checked."),
        )
        // A refusal Rust does not apply to the stored key (a record of another key) says nothing.
        assertNull(votingKeyCheckStatusText(state(MasternodeVotingKeyCheck.Invalid("x")), recheckUse, false, null))
        assertNull(votingKeyCheckStatusText(state(MasternodeVotingKeyCheck.Valid), provenUse, false, null))
    }

    // ── Checking when picked ─────────────────────────────────────────────

    private suspend fun checkPicked(
        answer: suspend () -> MasternodeVotingKeyCheck = { MasternodeVotingKeyCheck.Valid },
        stillCurrent: () -> Boolean = { true },
        identifier: String = identifierA,
    ): PickedVotingKeyCheck = checkPickedVotingKeyIfNeeded(
        database = db,
        identityId = identityId,
        networkRaw = testnet,
        identifier = identifier,
        rules = rules,
        check = { proTxHash, address ->
            checkedWith += proTxHash to address
            answer()
        },
        stillCurrent = stillCurrent,
    )

    private val checkedWith = mutableListOf<Pair<ByteArray, MasternodeVotingKeyAddress>>()

    @Test
    fun shouldNotCheckAKeyRustSaysNeedsNoCheck() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        rules.use = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = false)

        assertEquals(PickedVotingKeyCheck.NotNeeded, checkPicked())
        assertTrue(checkedWith.isEmpty())
    }

    /** The check reads the address from the identifier and records what Rust says. */
    @Test
    fun shouldCheckTheWireOrderProTxHashAndRecordRustsAnswer() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val recorded = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = recorded)

        assertEquals(PickedVotingKeyCheck.Recorded(MasternodeVotingKeyCheck.Valid), checkPicked())

        assertArrayEquals(identityId.reversedArray(), checkedWith.single().first)
        assertEquals(addressA, checkedWith.single().second)
        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.SameKey(null)), rules.asked)
        assertEquals(recorded, db.identityDao().getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /** A key replaced during the check is not given the old key's answer. */
    /** #6099: a shared masternode's row is checked for the pro_tx_hash it records. */
    @Test
    fun shouldCheckTheRecordedProTxHashOfASharedMasternode() = runBlocking {
        val proTxHash = ByteArray(32) { 0x5a }
        db.identityDao().upsert(row(identifierA).copy(masternodeProTxHash = proTxHash))

        checkPicked()

        assertArrayEquals(proTxHash, checkedWith.single().first)
    }

    @Test
    fun shouldDropAnAnswerAboutAKeyReplacedDuringTheCheck() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        rules.outcome = MasternodeVotingKeyCheckOutcome(
            true,
            MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA),
        )

        val result = checkPicked(answer = {
            db.identityDao().replaceVotingPrivateKeyIdentifier(identityId, testnet, identifierA, identifierB, 0L)
            MasternodeVotingKeyCheck.Valid
        })

        assertEquals(PickedVotingKeyCheck.Dropped, result)
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    @Test
    fun shouldDropAnAnswerFromAnotherNetworkOrSdk() = runBlocking {
        db.identityDao().upsert(row(identifierA))

        assertEquals(PickedVotingKeyCheck.Dropped, checkPicked(stillCurrent = { false }))
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)
        assertTrue(rules.asked.isEmpty())
    }

    /** A failed check learns nothing: the row is left as it was and the key may still vote. */
    @Test
    fun shouldLeaveTheRowAsItWasWhenTheCheckFails() = runBlocking {
        db.identityDao().upsert(row(identifierA))

        val result = checkPicked(answer = { throw IllegalStateException("offline") })

        assertEquals(PickedVotingKeyCheck.Failed("The voting key could not be checked: offline"), result)
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    /**
     * Recovery: a check in flight when the process dies (here: cancelled before it answers)
     * writes nothing, so the row does not look checked and the next pick checks it again.
     */
    @Test
    fun shouldWriteNothingForACheckCutShort() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        rules.outcome = MasternodeVotingKeyCheckOutcome(
            true,
            MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA),
        )
        val started = CompletableDeferred<Unit>()
        val never = CompletableDeferred<MasternodeVotingKeyCheck>()
        val running = async {
            checkPicked(answer = {
                started.complete(Unit)
                never.await()
            })
        }
        started.await()
        running.cancel()
        yield()

        assertTrue(running.isCancelled)
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)
        assertTrue(rules.asked.isEmpty())
    }

    @Test
    fun shouldNotCheckForARowThatNoLongerNamesThePickedKey() = runBlocking {
        db.identityDao().upsert(row(identifierB))

        assertEquals(PickedVotingKeyCheck.Dropped, checkPicked(identifier = identifierA))
        assertTrue(checkedWith.isEmpty())
    }

    private fun row(
        identifier: String?,
        recorded: MasternodeRecordedVotingKey? = null,
        id: Int = 0,
    ): IdentityEntity {
        val raw = when (recorded?.check) {
            null -> null
            MasternodeVotingKeyCheck.Valid -> "valid"
            is MasternodeVotingKeyCheck.Invalid -> "invalid"
            is MasternodeVotingKeyCheck.Undetermined -> "undetermined"
        }
        val message = when (val check = recorded?.check) {
            is MasternodeVotingKeyCheck.Invalid -> check.message
            is MasternodeVotingKeyCheck.Undetermined -> check.message
            else -> null
        }
        return IdentityEntity(
            identityId = if (id == 0) identityId else ByteArray(32) { id.toByte() },
            identityType = "Masternode",
            votingPrivateKeyIdentifier = identifier,
            networkRaw = testnet,
            votingKeyCheckRaw = raw,
            votingKeyCheckMessage = message,
            votingKeyCheckAddress = recorded?.votingAddress?.bytes,
        )
    }

    private fun hex(identity: IdentityEntity) = identity.identityId.joinToString("") { "%02x".format(it) }

    /** Returns what each test sets and records what it was asked; decides nothing itself. */
    private class StubRules : MasternodeVotingKeyRules {
        var outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = null)
        var use = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = true)
        val asked = mutableListOf<MasternodeVotingKeySlot>()
        val usedWith = mutableListOf<Pair<MasternodeRecordedVotingKey?, MasternodeVotingKeyAddress?>>()

        override fun outcome(
            check: MasternodeVotingKeyCheck,
            votingAddress: MasternodeVotingKeyAddress,
            slot: MasternodeVotingKeySlot,
        ): MasternodeVotingKeyCheckOutcome {
            asked += slot
            return outcome
        }

        override fun use(
            recorded: MasternodeRecordedVotingKey?,
            storedVotingAddress: MasternodeVotingKeyAddress?,
        ): MasternodeRecordedVotingKeyUse {
            usedWith += recorded to storedVotingAddress
            return use
        }
    }
}
