package org.dashfoundation.dashsdk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

/**
 * Pins the diagnosable file-logging install gate. The field log line
 * "SDK tracing file logging (INFO)… NOT installed (subscriber already set
 * or dir unwritable)" could not say WHICH condition failed; the gate now
 * separates them, and the unwritable check runs BEFORE any native call —
 * which is also what makes this JVM-testable (the .so cannot load here, so
 * reaching the native installer would throw).
 */
@RunWith(RobolectricTestRunner::class)
class SdkFileLoggingInstallTest {

    @get:Rule
    val tmp = TemporaryFolder()

    @Test
    fun shouldReportUnwritableSessionRootWithoutTouchingTheNativeInstaller() {
        // A regular FILE at the session-root path: mkdirs and the probe
        // write must both fail. Returning (rather than throwing
        // UnsatisfiedLinkError from the native loader) proves the gate ran
        // pre-native.
        val fileNotDir = tmp.newFile("not-a-directory")

        val outcome = Sdk.installFileLogging(
            level = Sdk.LogLevel.INFO,
            sessionRoot = fileNotDir.absolutePath,
        )

        assertEquals(Sdk.FileLoggingInstall.SESSION_ROOT_UNWRITABLE, outcome)
    }

    @Test
    fun shouldProbeWritableSessionRootTrue() {
        // An existing writable dir, and a nested not-yet-created one (the
        // installer is expected to create the session tree).
        assertTrue(Sdk.sessionRootWritable(tmp.root))
        assertTrue(Sdk.sessionRootWritable(File(tmp.root, "nested/session")))
    }

    @Test
    fun shouldProbeFileAsSessionRootFalse() {
        assertFalse(Sdk.sessionRootWritable(tmp.newFile("plain-file")))
    }

    @Test
    fun shouldNotDeleteACallerOwnedEntryNamedLikeTheProbe() {
        // The probe once used the fixed name `.dash_sdk_write_probe` and
        // deleted that path first — destroying a caller-owned file of the
        // same name in the caller-selected session root (PR review). The
        // probe must be uniquely named and delete only what it created.
        val callerOwned = File(tmp.root, ".dash_sdk_write_probe")
        callerOwned.writeText("caller data")

        assertTrue(Sdk.sessionRootWritable(tmp.root))

        assertTrue(callerOwned.exists())
        assertEquals("caller data", callerOwned.readText())
    }

    @Test
    fun shouldLeaveNoProbeResidueBehind() {
        assertTrue(Sdk.sessionRootWritable(tmp.root))

        val leftovers = tmp.root.walkTopDown()
            .filter { it.isFile && it.name.startsWith(".dash_sdk_write_probe") }
            .toList()
        assertEquals(emptyList<File>(), leftovers)
    }

    @Test
    fun shouldReportABlockedFixedLogDestinationWithoutTouchingTheNativeInstaller() {
        // The ALREADY_SET misattribution shape (PR review): the root itself
        // probes writable, but the native create_dir_all("dash_sdk") would
        // fail on this regular FILE — previously reported as ALREADY_SET
        // with no subscriber in sight. Returning SESSION_ROOT_UNWRITABLE
        // (rather than throwing UnsatisfiedLinkError from the native
        // loader) also proves the check ran pre-native.
        tmp.newFile("dash_sdk")

        val outcome = Sdk.installFileLogging(
            level = Sdk.LogLevel.INFO,
            sessionRoot = tmp.root.absolutePath,
        )

        assertEquals(Sdk.FileLoggingInstall.SESSION_ROOT_UNWRITABLE, outcome)
    }

    @Test
    fun shouldNameTheBlockedDestinationNotJustTheRoot() {
        tmp.newFile("platform_wallet")

        assertEquals(
            File(tmp.root, "platform_wallet"),
            Sdk.firstUnwritableLogDestination(tmp.root),
        )
    }

    @Test
    fun shouldRejectADirectorySquattingOnADestinationFile() {
        // create(true).append(true).open on a path that is a directory
        // fails natively; the probe must catch it up front.
        File(tmp.root, "dash_sdk/run.log").mkdirs()

        assertEquals(
            File(tmp.root, "dash_sdk/run.log"),
            Sdk.firstUnwritableLogDestination(tmp.root),
        )
    }
}
