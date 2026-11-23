package org.dashfoundation.dashsdk.voting

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The marshalling and persistence of the masternode voting key check (port of Swift
 * `SDK+MasternodeVotingKeyCheck.swift` and the check fields of `PersistentIdentity.swift`).
 * The rules are rs-sdk's, tested there; the JSON these decode is the shape
 * `rs-unified-sdk-jni`'s `should_write_the_json_the_kotlin_decoder_reads` pins, and the JNI
 * bindings are exercised by `FfiSmokeTest.masternodeVotingKeyCheckSymbolsBindAndRejectInvalidInput`.
 * The rules here are a recorder that returns what each test sets, never a copy of Rust's.
 */
@RunWith(RobolectricTestRunner::class)
class MasternodeVotingKeyChecksTest {

    private val testnet = 1
    private val mainnet = 0
    private val identityId = ByteArray(32) { (it + 1).toByte() }
    private val addressA = MasternodeVotingKeyAddress.of(ByteArray(20) { 0x0a })!!
    private val addressB = MasternodeVotingKeyAddress.of(ByteArray(20) { 0x0b })!!
    private val identifierA = MasternodeVotingKeyStore.identifierFor(identityId, addressA.bytes)
    private val identifierB = MasternodeVotingKeyStore.identifierFor(identityId, addressB.bytes)

    private lateinit var db: DashDatabase

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    // ── JSON from the JNI exports ────────────────────────────────────────

    @Test
    fun shouldDecodeEachVerdictWithItsMessage() {
        assertEquals(
            MasternodeVotingKeyCheck.Valid,
            MasternodeVotingKeyCheckCodec.decodeCheck("""{"verdict":0,"message":null}"""),
        )
        assertEquals(
            MasternodeVotingKeyCheck.Invalid("no \"key\""),
            MasternodeVotingKeyCheckCodec.decodeCheck("""{"message":"no \"key\"","verdict":1}"""),
        )
        assertEquals(
            MasternodeVotingKeyCheck.Undetermined("timeout"),
            MasternodeVotingKeyCheckCodec.decodeCheck("""{"verdict":2,"message":"timeout"}"""),
        )
    }

    @Test
    fun shouldRefuseAnUnknownVerdictOrNoAnswer() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeVotingKeyCheckCodec.decodeCheck("""{"verdict":3,"message":null}""")
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeVotingKeyCheckCodec.decodeCheck(null)
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeVotingKeyCheckCodec.decodeCheck("not json")
        }
    }

    /** A record of the check's own kind keeps the check's message (Swift `recordedKey`). */
    @Test
    fun shouldDecodeAnOutcomeKeepingTheChecksMessage() {
        val address = "0a".repeat(20)
        val undetermined = MasternodeVotingKeyCheck.Undetermined("timeout")
        assertEquals(
            MasternodeVotingKeyCheckOutcome(true, MasternodeRecordedVotingKey(undetermined, addressA)),
            MasternodeVotingKeyCheckCodec.decodeOutcome(
                """{"storeKey":true,"record":{"check":3,"votingAddress":"$address"}}""",
                preferring = undetermined,
            ),
        )
        // A refusal recorded for a check of another kind carries no message.
        assertEquals(
            MasternodeVotingKeyCheckOutcome(
                false,
                MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid(""), addressA),
            ),
            MasternodeVotingKeyCheckCodec.decodeOutcome(
                """{"storeKey":false,"record":{"check":2,"votingAddress":"$address"}}""",
                preferring = MasternodeVotingKeyCheck.Valid,
            ),
        )
        assertEquals(
            MasternodeVotingKeyCheckOutcome(true, null),
            MasternodeVotingKeyCheckCodec.decodeOutcome(
                """{"storeKey":true,"record":null}""",
                preferring = undetermined,
            ),
        )
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeVotingKeyCheckCodec.decodeOutcome(
                """{"storeKey":true,"record":{"check":1,"votingAddress":"0a"}}""",
                preferring = MasternodeVotingKeyCheck.Valid,
            )
        }
    }

    @Test
    fun shouldDecodeTheUseOfAStoredKey() {
        assertEquals(
            MasternodeRecordedVotingKeyUse(canVote = false, checkBeforeUse = true),
            MasternodeVotingKeyCheckCodec.decodeUse("""{"canVote":false,"checkBeforeUse":true}"""),
        )
        assertThrows(DashSdkError.SerializationError::class.java) {
            MasternodeVotingKeyCheckCodec.decodeUse("""{"canVote":true}""")
        }
    }

    /** The C enum values the JNI exports read, as `rs-sdk-ffi`'s `voting_key_check.rs` declares. */
    @Test
    fun shouldPassTheCEnumValues() {
        val codec = MasternodeVotingKeyCheckCodec
        assertEquals(0, codec.verdictOf(MasternodeVotingKeyCheck.Valid))
        assertEquals(1, codec.verdictOf(MasternodeVotingKeyCheck.Invalid("x")))
        assertEquals(2, codec.verdictOf(MasternodeVotingKeyCheck.Undetermined("x")))
        assertEquals(0, codec.slotOf(MasternodeVotingKeySlot.Empty))
        assertEquals(1, codec.slotOf(MasternodeVotingKeySlot.SameKey(null)))
        assertEquals(2, codec.slotOf(MasternodeVotingKeySlot.DifferentKey))
        assertEquals(0, codec.recordedCheckOf(null))
        assertEquals(1, codec.recordedCheckOf(MasternodeVotingKeyCheck.Valid))
        assertEquals(2, codec.recordedCheckOf(MasternodeVotingKeyCheck.Invalid("x")))
        assertEquals(3, codec.recordedCheckOf(MasternodeVotingKeyCheck.Undetermined("x")))
    }

    // ── The row ──────────────────────────────────────────────────────────

    @Test
    fun shouldReadTheStoredKeysAddressFromItsIdentifierWithoutAKey() {
        assertEquals(addressA, row(identifierA).storedVotingKeyAddress)
        assertNull(row(null).storedVotingKeyAddress)
        assertNull(row("not an identifier").storedVotingKeyAddress)
        assertNull(MasternodeVotingKeyAddress.of(ByteArray(19)))
    }

    /** A check recorded without a whole address is nothing recorded, as Swift reads it. */
    @Test
    fun shouldReadARecordOnlyWithItsAddress() {
        val withAddress = row(identifierA).copy(
            votingKeyCheckRaw = "invalid",
            votingKeyCheckMessage = "rotated",
            votingKeyCheckAddress = addressA.bytes,
        )
        assertEquals(
            MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressA),
            withAddress.recordedVotingKey,
        )
        assertEquals(MasternodeVotingKeyCheck.Invalid("rotated"), withAddress.votingKeyCheck)
        assertNull(withAddress.copy(votingKeyCheckAddress = null).recordedVotingKey)
        assertNull(withAddress.copy(votingKeyCheckAddress = ByteArray(5)).recordedVotingKey)
        assertNull(withAddress.copy(votingKeyCheckRaw = "something newer").recordedVotingKey)
    }

    // ── Recording what Rust says ─────────────────────────────────────────

    @Test
    fun shouldWriteTheRecordRustReturnsWithItsAddress() = runBlocking {
        db.identityDao().upsert(row(identifierA))
        val check = MasternodeVotingKeyCheck.Undetermined("timeout")
        val rules = RecordingRules(
            MasternodeVotingKeyCheckOutcome(true, MasternodeRecordedVotingKey(check, addressA)),
        )

        val outcome = MasternodeVotingKeyRecords.record(
            db, identityId, testnet, identifierA, check,
            MasternodeVotingKeyRecords.SlotBefore.EMPTY, rules,
        )

        assertEquals(rules.answer, outcome)
        assertEquals(listOf(Triple(check, addressA, MasternodeVotingKeySlot.Empty)), rules.asked)
        val stored = db.identityDao().getByIdentityId(identityId)!!
        assertEquals("undetermined", stored.votingKeyCheckRaw)
        assertEquals("timeout", stored.votingKeyCheckMessage)
        assertArrayEquals(addressA.bytes, stored.votingKeyCheckAddress)
    }

    /** The same key is reported to Rust with the record the row holds now. */
    @Test
    fun shouldReportTheSameKeyWithItsCurrentRecord() = runBlocking {
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA)
        db.identityDao().upsert(
            row(identifierA).copy(votingKeyCheckRaw = "valid", votingKeyCheckAddress = addressA.bytes),
        )
        val rules = RecordingRules(MasternodeVotingKeyCheckOutcome(true, null))

        MasternodeVotingKeyRecords.record(
            db, identityId, testnet, identifierA, MasternodeVotingKeyCheck.Undetermined("down"),
            MasternodeVotingKeyRecords.SlotBefore.SAME_KEY, rules,
        )
        MasternodeVotingKeyRecords.record(
            db, identityId, testnet, identifierA, MasternodeVotingKeyCheck.Valid,
            MasternodeVotingKeyRecords.SlotBefore.DIFFERENT_KEY, rules,
        )

        assertEquals(MasternodeVotingKeySlot.SameKey(valid), rules.asked[0].third)
        assertEquals(MasternodeVotingKeySlot.DifferentKey, rules.asked[1].third)
        // A null record leaves what is recorded unchanged.
        assertEquals(valid, db.identityDao().getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /**
     * An answer about a key the row no longer names (replaced during the check), or for a row
     * gone or on another network, is dropped without asking Rust.
     */
    @Test
    fun shouldDropAnAnswerAboutAKeyTheRowNoLongerNames() = runBlocking {
        val rules = RecordingRules(
            MasternodeVotingKeyCheckOutcome(
                true,
                MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressA),
            ),
        )
        suspend fun record(network: Int = testnet) = MasternodeVotingKeyRecords.record(
            db, identityId, network, identifierA, MasternodeVotingKeyCheck.Valid,
            MasternodeVotingKeyRecords.SlotBefore.SAME_KEY, rules,
        )

        assertNull("no row", record())
        db.identityDao().upsert(row(identifierB))
        assertNull("another key", record())
        db.identityDao().upsert(row(identifierA, network = mainnet))
        assertNull("another network", record())
        assertEquals(emptyList<Any>(), rules.asked)
        assertNull(db.identityDao().getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    /**
     * Recovery: a key stored by a load that died before its check was recorded keeps whatever
     * the row held, which is either nothing or a record of the previous key's address. Rust
     * reads both as never checked for the stored key, so it is checked when picked, and
     * nothing looks checked that was not.
     */
    @Test
    fun shouldLeaveAReplacedKeyUncheckedWhenItsRecordWasNeverWritten() = runBlocking {
        db.identityDao().upsert(
            row(identifierA).copy(votingKeyCheckRaw = "valid", votingKeyCheckAddress = addressA.bytes),
        )
        // The replacement's row write, then the process dies before the record write.
        db.identityDao().replaceVotingPrivateKeyIdentifier(identityId, testnet, identifierA, identifierB, 0L)

        val stored = db.identityDao().getByIdentityId(identityId)!!
        assertEquals(addressB, stored.storedVotingKeyAddress)
        // The only record left is made of another key.
        assertEquals(addressA, stored.recordedVotingKey?.votingAddress)
        val rules = RecordingRules(MasternodeVotingKeyCheckOutcome(true, null))
        stored.storedVotingKeyUse(rules)
        assertEquals(addressB, rules.usedWith.single().second)
    }

    private fun row(identifier: String?, network: Int = testnet) = IdentityEntity(
        identityId = identityId,
        identityType = "Masternode",
        votingPrivateKeyIdentifier = identifier,
        networkRaw = network,
    )

    /** Records what it was asked and returns [answer]; never decides anything itself. */
    private class RecordingRules(val answer: MasternodeVotingKeyCheckOutcome) : MasternodeVotingKeyRules {
        val asked = mutableListOf<Triple<MasternodeVotingKeyCheck, MasternodeVotingKeyAddress, MasternodeVotingKeySlot>>()
        val usedWith = mutableListOf<Pair<MasternodeRecordedVotingKey?, MasternodeVotingKeyAddress?>>()

        override fun outcome(
            check: MasternodeVotingKeyCheck,
            votingAddress: MasternodeVotingKeyAddress,
            slot: MasternodeVotingKeySlot,
        ): MasternodeVotingKeyCheckOutcome {
            asked += Triple(check, votingAddress, slot)
            return answer
        }

        override fun use(
            recorded: MasternodeRecordedVotingKey?,
            storedVotingAddress: MasternodeVotingKeyAddress?,
        ): MasternodeRecordedVotingKeyUse {
            usedWith += recorded to storedVotingAddress
            return MasternodeRecordedVotingKeyUse(canVote = true, checkBeforeUse = true)
        }
    }
}
