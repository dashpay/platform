package org.dashfoundation.example.ui.voting

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.first
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
        // No stored key cannot vote, and Rust is not asked about it.
        assertTrue(isVotingKeyRefused(voterKeyState(row(null)), rules))
        assertTrue(isVotingKeyRefused(null, rules))
        assertEquals(1, rules.usedWith.size)
    }

    @Test
    fun shouldLabelARefusedCandidate() {
        val identity = row(identifierA).copy(alias = "mn")
        assertEquals(
            "mn (Masternode, voting key does not match)",
            voterLabel(identity, listOf(identity), StoredVoterDecision.REFUSED),
        )
        // A key this device cannot read is not reported as a Platform refusal.
        assertEquals(
            "mn (Masternode, voting key cannot be read)",
            voterLabel(identity, listOf(identity), StoredVoterDecision.UNREADABLE),
        )
        assertEquals("mn (Masternode)", voterLabel(identity, listOf(identity), StoredVoterDecision.CASTABLE))
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
            selectedCastable = !refused,
            allowsTypedProTxHash = false,
            useTypedProTxHash = false,
            proTxHashText = "",
            debugBuild = false,
            useTypedCredentials = false,
            votingKeyText = "",
        )
        assertNull(publish(refused = true))
        assertEquals(MasternodeVoter.StoredKey(identityId, identifierA), publish(refused = false))
    }

    // ── Whether the picked key may be published ─────────────────────────

    private val awaitingCheck = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = true)
    private val proven = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = false)
    private val refusedUse = MasternodeRecordedVotingKeyUse(canVote = false, checkBeforeUse = false)

    /**
     * A key Rust says to check first is not castable until a check of that same key finished
     * in this dialog; an unanswered check still lets it vote, as Swift does.
     */
    @Test
    fun shouldHoldAKeyThatNeedsACheckUntilACheckOfThatKeyFinished() {
        val state = voterKeyState(row(identifierA))
        rules.use = awaitingCheck

        assertEquals(StoredVoterDecision.AWAITING_CHECK, storedVoterDecision(state, null, rules))
        // A check that finished for another key does not count.
        assertEquals(
            StoredVoterDecision.AWAITING_CHECK,
            storedVoterDecision(state, PickedKeyCheckState(identifierB), rules),
        )
        assertEquals(
            StoredVoterDecision.CASTABLE,
            storedVoterDecision(state, PickedKeyCheckState(identifierA), rules),
        )
        rules.use = proven
        assertEquals(StoredVoterDecision.CASTABLE, storedVoterDecision(state, null, rules))
    }

    /** A refusal that could not be saved is applied anyway: the key stays refused. */
    @Test
    fun shouldApplyARefusalThatCouldNotBeSaved() {
        val state = voterKeyState(row(identifierA))
        val unsaved = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        rules.use = refusedUse

        assertEquals(
            StoredVoterDecision.REFUSED,
            storedVoterDecision(state, PickedKeyCheckState(identifierA, unsaved), rules),
        )
        assertEquals(unsaved to addressA, rules.usedWith.last())
    }

    @Test
    fun shouldNeverPublishAnUnreadableIdentifier() {
        val state = voterKeyState(row("mnvoting.x"))

        assertEquals(StoredVoterDecision.UNREADABLE, storedVoterDecision(state, null, rules))
        assertTrue(isVotingKeyRefused(state, rules))
        assertTrue(rules.usedWith.isEmpty())
        assertEquals(
            UNREADABLE_VOTING_KEY_IDENTIFIER_MESSAGE to true,
            votingKeyCheckStatusText(state, null, false, null),
        )
    }

    /** An answer that could not be saved keeps what Rust said to record, for the section. */
    @Test
    fun shouldKeepRustsRecordForAnAnswerThatCouldNotBeSaved() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val refusal = MasternodeVotingKeyCheck.Invalid("rotated")
        val unsaved = MasternodeRecordedVotingKey(refusal, addressA)
        rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = false, record = unsaved)

        // The row stops accepting writes while Platform answers, so the record write throws.
        val result = checkPicked(answer = {
            db.openHelper.writableDatabase.execSQL(
                "CREATE TRIGGER refuse_writes BEFORE UPDATE ON identities " +
                    "BEGIN SELECT RAISE(ABORT, 'disk full'); END",
            )
            refusal
        })

        assertTrue(result is PickedVotingKeyCheck.Failed)
        assertEquals(unsaved, (result as PickedVotingKeyCheck.Failed).unsaved)
    }

    /**
     * Recovery: after a check cut short (process death) wrote nothing, the next pick checks the
     * stored key's address and records the answer.
     */
    @Test
    fun shouldCheckAndRecordOnTheNextPickAfterACheckWasCutShort() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val recorded = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = recorded)
        val started = CompletableDeferred<Unit>()
        val cutShort = async {
            checkPicked(answer = {
                started.complete(Unit)
                CompletableDeferred<MasternodeVotingKeyCheck>().await()
            })
        }
        started.await()
        cutShort.cancel()
        yield()
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)

        assertEquals(PickedVotingKeyCheck.Recorded(MasternodeVotingKeyCheck.Valid, recorded, 1), checkPicked())
        assertEquals(addressA, checkedWith.last().second)
        assertEquals(recorded, db.identityDao().getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /**
     * Recovery across a relaunch: a load stored key B over A in a file-backed database and the
     * process died before recording B's check. The database is reopened, the picker's own
     * projection of the recovered row ([voterRows]) gives the identifier and the decision, and
     * the next pick checks B's address and records the answer.
     */
    @Test
    fun shouldCheckAReplacedKeyOnTheNextPickAfterALoadDiedBeforeRecording() = runBlocking {
        val context = RuntimeEnvironment.getApplication()
        context.deleteDatabase(DashDatabase.DATABASE_NAME)
        val before = DashDatabase.create(context)
        before.identityDao().upsert(
            row(identifierA, MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)),
        )
        // The replacement's row write, then the process dies before the record write.
        before.identityDao().replaceVotingPrivateKeyIdentifier(identityId, testnet, identifierA, identifierB, 0L)
        before.close()

        val reopened = DashDatabase.create(context)
        try {
            val recovered = voterRows(reopened.identityDao().observeByNetwork(testnet).first()).single()
            assertEquals(identifierB, recovered.state.identifier)
            // The record left by A does not count for B: Rust is asked with B's address, and
            // the key is held until it is checked.
            rules.use = MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = true)
            assertEquals(StoredVoterDecision.AWAITING_CHECK, storedVoterDecision(recovered.state, null, rules))
            assertEquals(addressB, rules.usedWith.last().second)

            val recordedB = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressB)
            rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = recordedB)
            assertEquals(
                PickedVotingKeyCheck.Recorded(MasternodeVotingKeyCheck.Valid, recordedB, 1),
                checkPicked(identifier = recovered.state.identifier!!, database = reopened),
            )
            assertEquals(addressB, checkedWith.single().second)
            assertEquals(recordedB, reopened.identityDao().getByIdentityId(identityId)!!.recordedVotingKey)
        } finally {
            reopened.close()
            context.deleteDatabase(DashDatabase.DATABASE_NAME)
        }
    }

    /** [voterKeyState] of a row whose record was written [revision] times. */
    private fun stateAt(revision: Long, recorded: MasternodeRecordedVotingKey? = null) =
        voterKeyState(row(identifierA, recorded).copy(votingKeyCheckRevision = revision))

    /**
     * A result this dialog saved stands until Room shows that write or a later one (the row's
     * revision reaches the one the write left); then Room decides, so a newer refusal of the
     * same key written by another check counts. A result that could not be saved stands.
     */
    @Test
    fun shouldLetANewerRefusalInRoomSupersedeASavedResult() {
        val undetermined = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val saved = PickedKeyCheckState(identifierA, undetermined, savedRevision = 1)

        // Before Room delivers the write, the dialog's result speaks.
        assertEquals(undetermined, effectiveRecord(stateAt(0), saved))
        // Room shows this write.
        assertEquals(undetermined, effectiveRecord(stateAt(1, undetermined), saved))
        // Room shows a newer refusal of the same key: it decides.
        assertEquals(refusal, effectiveRecord(stateAt(2, refusal), saved))
        rules.use = refusedUse
        assertEquals(StoredVoterDecision.REFUSED, storedVoterDecision(stateAt(2, refusal), saved, rules))

        // A refusal that could not be saved is never retired by Room's record.
        val unsaved = PickedKeyCheckState(identifierA, refusal, savedRevision = null)
        assertEquals(refusal, effectiveRecord(stateAt(5, undetermined), unsaved))
    }

    /**
     * An older record Room delivers after the dialog saved its refusal (another check's write,
     * at a lower revision) does not release the key.
     */
    @Test
    fun shouldKeepASavedRefusalOverAnOlderRecordRoomDeliversLate() {
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val olderUndetermined = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        val saved = PickedKeyCheckState(identifierA, refusal, savedRevision = 2)
        rules.use = refusedUse

        assertEquals(refusal, effectiveRecord(stateAt(1, olderUndetermined), saved))
        assertEquals(StoredVoterDecision.REFUSED, storedVoterDecision(stateAt(1, olderUndetermined), saved, rules))
        assertEquals(refusal to addressA, rules.usedWith.last())
    }

    /**
     * Only what the dialog learned reaches the check, and only while the row it reads is behind
     * it: a refusal the row already holds at a newer revision decides, with no Platform call.
     */
    @Test
    fun shouldNotCheckAKeyTheFreshRowAlreadyRefuses() = runBlocking {
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val undetermined = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        db.identityDao().upsert(row(identifierA, refusal).copy(votingKeyCheckRevision = 3))
        rules.use = refusedUse

        val result = checkPicked(picked = PickedKeyCheckState(identifierA, undetermined, savedRevision = 2))

        // The refusal the helper read, at its revision, goes back to the picker.
        assertEquals(PickedVotingKeyCheck.NotNeeded(refusal, 3), result)
        assertTrue(checkedWith.isEmpty())
        assertEquals(refusal to addressA, rules.usedWith.single())
        // The production transition holds it as the dialog's record until Room delivers
        // revision 3, so no voter is published against the older displayed row (revision 2,
        // undetermined): Rust is asked about the refusal, not the stale record.
        val held = nextPickedState(null, identifierA, result, revisionAtStart = 2)!!
        assertEquals(
            StoredVoterDecision.REFUSED,
            storedVoterDecision(stateAt(2, undetermined), held, rules),
        )
        assertEquals(refusal to addressA, rules.usedWith.last())
    }

    /**
     * A finished check counts only against the record it was made for: once another check or a
     * load writes a newer record of the same key, a key Rust says to check is held again until
     * a check of that record finishes (Swift, after #6145).
     */
    @Test
    fun shouldHoldAKeyAgainWhenItsRecordChangesAfterTheCheck() {
        rules.use = awaitingCheck
        val undetermined = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        // The check left `undetermined` at revision 2; a sibling write turns a newer record
        // into another verdict at revision 3.
        val picked = PickedKeyCheckState(identifierA, undetermined, finished = true, savedRevision = 2, checkedRevision = 2)
        val newer = stateAt(3, valid)

        assertEquals(StoredVoterDecision.CASTABLE, storedVoterDecision(stateAt(2, undetermined), picked, rules))
        assertEquals(StoredVoterDecision.AWAITING_CHECK, storedVoterDecision(newer, picked, rules))
        assertTrue(checkFinishedFor(stateAt(1), picked))
        assertFalse(checkFinishedFor(newer, picked))
    }

    /**
     * Two open pickers of the same key: a newer write of the same inconclusive verdict is not a
     * new record, so the finished check still counts and no retry is requested.
     */
    @Test
    fun shouldSettleOnARepeatedInconclusiveVerdict() {
        val undeterminedA = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        val undeterminedB = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("timeout"), addressA)
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val picked = PickedKeyCheckState(identifierA, undeterminedA, savedRevision = 2, checkedRevision = 2)

        assertTrue(sameVerdict(undeterminedA, undeterminedB))
        assertFalse(sameVerdict(undeterminedA, refusal))
        assertTrue(checkFinishedFor(stateAt(3, undeterminedB), picked))
        assertFalse(checkFinishedFor(stateAt(3, refusal), picked))
    }

    /**
     * At vote time a newer record Rust says to check first refuses the vote unless it repeats
     * the verdict the section cleared; the cleared record itself still votes.
     */
    @Test
    fun shouldRefuseAVoteClearedOnAnOlderRecordThatMustBeCheckedAgain() {
        val undetermined = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Undetermined("down"), addressA)
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        val voter = MasternodeVoter.StoredKey(identityId, identifierA, clearedRevision = 2, clearedRecord = valid)

        assertTrue(clearedRecordStillHolds(voter, 2, valid))
        assertFalse(clearedRecordStillHolds(voter, 3, undetermined))
        val repeat = MasternodeVoter.StoredKey(identityId, identifierA, clearedRevision = 2, clearedRecord = undetermined)
        assertTrue(clearedRecordStillHolds(repeat, 3, undetermined))
    }

    /**
     * A pick counts as checked only when its result was saved (or no check was needed, or
     * Platform could not answer). A confirmation that could not be saved grants nothing even
     * when Rust says such a record may vote without a check; the next pick checks it again.
     */
    @Test
    fun shouldKeepAKeyUnpublishedWhenItsConfirmationCouldNotBeSaved() = runBlocking {
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        val unsaved = nextPickedState(
            null, identifierA, PickedVotingKeyCheck.Failed("disk full", valid), revisionAtStart = 1,
        )!!
        // Rust's flags for a valid record: may vote, no check needed.
        rules.use = proven

        assertTrue(unsaved.saveFailed)
        assertEquals(StoredVoterDecision.AWAITING_CHECK, storedVoterDecision(stateAt(1), unsaved, rules))

        // The next pick retries the check and, once it is saved, releases the key.
        db.identityDao().upsert(row(identifierA))
        rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = valid)
        val retried = checkPicked(picked = unsaved.copy(finished = false))
        assertTrue(retried is PickedVotingKeyCheck.Recorded)
        assertEquals(addressA, checkedWith.single().second)
        val saved = nextPickedState(unsaved, identifierA, retried, revisionAtStart = 0)!!
        assertEquals(
            StoredVoterDecision.CASTABLE,
            storedVoterDecision(voterKeyState(db.identityDao().getByIdentityId(identityId)!!), saved, rules),
        )

        // The check's call threw: nothing was learned, so the pick stays unchecked and the key
        // is not offered (Swift `.notFinished`, after #6145).
        val threw = nextPickedState(
            null, identifierA, PickedVotingKeyCheck.Failed("timeout"), revisionAtStart = 1,
        )!!
        rules.use = awaitingCheck
        assertFalse(threw.finished)
        assertEquals(StoredVoterDecision.AWAITING_CHECK, storedVoterDecision(stateAt(1), threw, rules))
    }

    /** An unsaved refusal still refuses (Rust: may not vote) and is not checked again. */
    @Test
    fun shouldKeepAnUnsavedRefusalWithoutRetrying() = runBlocking {
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        val unsaved = nextPickedState(
            null, identifierA, PickedVotingKeyCheck.Failed("disk full", refusal), revisionAtStart = 0,
        )!!
        rules.use = refusedUse
        db.identityDao().upsert(row(identifierA))

        assertEquals(StoredVoterDecision.REFUSED, storedVoterDecision(stateAt(0), unsaved, rules))
        assertTrue(checkPicked(picked = unsaved) is PickedVotingKeyCheck.NotNeeded)
        assertTrue(checkedWith.isEmpty())
    }

    /**
     * The proposal dialogs key their answer on the masternode: a voter republished with a newer
     * clearance keeps it, another masternode does not.
     */
    @Test
    fun shouldKeepTheAnswerTargetAcrossAClearanceOnlyUpdate() {
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        val before = MasternodeVoter.StoredKey(identityId, identifierA, clearedRevision = 1, clearedRecord = valid)
        val after = MasternodeVoter.StoredKey(identityId, identifierA, clearedRevision = 2, clearedRecord = valid)
        val other = MasternodeVoter.StoredKey(ByteArray(32) { 9 }, identifierA)

        assertFalse(before == after)
        assertEquals(answerTargetOf(before), answerTargetOf(after))
        assertFalse(answerTargetOf(before) == answerTargetOf(other))
        assertNull(answerTargetOf(null))
    }

    /** A check started again for a re-picked key holds it until that check finishes. */
    @Test
    fun shouldHoldAReCheckedKeyUntilTheNewCheckFinishes() {
        val state = voterKeyState(row(identifierA))
        rules.use = awaitingCheck

        assertEquals(
            StoredVoterDecision.AWAITING_CHECK,
            storedVoterDecision(state, PickedKeyCheckState(identifierA, finished = false), rules),
        )
    }

    /**
     * A refusal learned in this dialog (saved or not) is weighed in place of the row's record,
     * so returning to the key does not check it again or let an inconclusive retry clear it.
     */
    @Test
    fun shouldNotCheckAwayARefusalLearnedInThisDialog() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val refusal = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA)
        rules.use = refusedUse

        assertTrue(checkPicked(picked = PickedKeyCheckState(identifierA, refusal)) is PickedVotingKeyCheck.NotNeeded)
        assertTrue(checkedWith.isEmpty())
        assertEquals(refusal to addressA, rules.usedWith.single())
        // And the picker keeps it refused.
        assertEquals(
            StoredVoterDecision.REFUSED,
            storedVoterDecision(voterKeyState(row(identifierA)), PickedKeyCheckState(identifierA, refusal), rules),
        )
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
            "Not checked. It cannot vote until a check of it finishes: pick this identity again." to true,
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
        picked: PickedKeyCheckState? = null,
        database: DashDatabase = db,
    ): PickedVotingKeyCheck = checkPickedVotingKeyIfNeeded(
        database = database,
        identityId = identityId,
        networkRaw = testnet,
        identifier = identifier,
        rules = rules,
        picked = picked,
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

        assertTrue(checkPicked() is PickedVotingKeyCheck.NotNeeded)
        assertTrue(checkedWith.isEmpty())
    }

    /** The check reads the address from the identifier and records what Rust says. */
    @Test
    fun shouldCheckTheWireOrderProTxHashAndRecordRustsAnswer() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val recorded = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        rules.outcome = MasternodeVotingKeyCheckOutcome(storeKey = true, record = recorded)

        assertEquals(PickedVotingKeyCheck.Recorded(MasternodeVotingKeyCheck.Valid, recorded, 1), checkPicked())

        assertArrayEquals(identityId.reversedArray(), checkedWith.single().first)
        assertEquals(addressA, checkedWith.single().second)
        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.SameKey(null)), rules.asked)
        assertEquals(recorded, db.identityDao().getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /** A key replaced during the check is not given the old key's answer. */
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
