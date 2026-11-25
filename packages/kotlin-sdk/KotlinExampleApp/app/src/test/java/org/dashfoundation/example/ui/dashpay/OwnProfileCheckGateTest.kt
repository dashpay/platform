package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * When Your Profile may treat a miss in the wallet's own-profile cache as
 * "no profile" ([OwnProfileCheckGate]). No JNI or network: the gate only
 * sees the manager's last completed DashPay pass time.
 */
class OwnProfileCheckGateTest {

    @Test
    fun shouldWaitForASecondPassAfterAnOldBaseline() {
        val gate = OwnProfileCheckGate()
        assertFalse("baseline", gate.observe(1_000))
        assertFalse("same pass", gate.observe(1_000))
        assertFalse("the pass running at open may predate the identity", gate.observe(1_015))
        assertTrue("the next pass started after the screen opened", gate.observe(1_030))
        assertTrue("stays passed", gate.observe(1_030))
    }

    @Test
    fun shouldCountPassesWithoutABusyFlagOrRoomWrite() {
        // Only the pass time changes; the gate needs nothing else. A real
        // zero (no pass recorded yet) is a valid baseline.
        val gate = OwnProfileCheckGate()
        gate.observe(0)
        assertFalse(gate.observe(500))
        assertTrue(gate.observe(515))
    }

    @Test
    fun shouldNotCountAZeroOrARestoredValueAsAPass() {
        val gate = OwnProfileCheckGate()
        gate.observe(1_000)
        assertFalse("zero is no completion", gate.observe(0))
        assertFalse("the baseline again is no new pass", gate.observe(1_000))
        assertFalse(gate.observe(1_015))
        assertFalse(gate.observe(0))
        assertTrue(gate.observe(1_030))
    }

    @Test
    fun shouldCountPassesAfterTheClockMovesBackward() {
        val gate = OwnProfileCheckGate()
        gate.observe(2_000)
        assertFalse(gate.observe(1_500))
        assertTrue("change, not order, counts", gate.observe(1_515))
    }

    @Test
    fun shouldNotTakeAFailedReadAsTheBaseline() {
        val gate = OwnProfileCheckGate()
        assertFalse("a failed read is unknown", gate.observe(null))
        assertFalse("first real value is the baseline", gate.observe(500))
        assertFalse(gate.observe(515))
        assertFalse(gate.observe(null))
        assertTrue(gate.observe(530))
    }

    @Test
    fun shouldPollUntilASecondPassAfterOpenWithNothingElseChanging() = runTest {
        // The profile stays a miss and only the pass time moves, as when a
        // pass finds no profile, writes no Room row and the busy flag is
        // never sampled.
        var lastSync = 1_000L
        val checker = OwnProfileChecker(readProfile = { null }, readLastSync = { lastSync })
        val answers = mutableListOf<OwnProfileChecker.Answer>()
        val job = launch { checker.pollUntilChecked(intervalMs = 2_000) { answers += it } }
        runCurrent()
        assertFalse(answers.last().checked)

        lastSync = 1_015
        advanceTimeBy(2_000)
        runCurrent()
        assertFalse("one pass may predate the screen", answers.last().checked)

        lastSync = 1_030
        advanceTimeBy(2_000)
        runCurrent()
        assertTrue(answers.last().checked)
        assertEquals(null, answers.last().raw)
        assertTrue("polling stops once checked", job.isCompleted)
    }

    @Test
    fun shouldAnswerAtOnceOnACacheHit() = runTest {
        val checker = OwnProfileChecker(readProfile = { "{\"displayName\":\"Alice\"}" }, readLastSync = { 1_000L })
        val answers = mutableListOf<OwnProfileChecker.Answer>()
        checker.pollUntilChecked { answers += it }
        assertEquals(1, answers.size)
        assertTrue(answers.single().checked)
    }
}
