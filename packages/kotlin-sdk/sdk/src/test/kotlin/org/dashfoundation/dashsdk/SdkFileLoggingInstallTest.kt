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
}
