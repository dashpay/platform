package org.dashfoundation.dashsdk.wallet

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Source-scanning regression guard for the teardown-fence contract: every
 * `suspend fun` in the SDK that borrows a raw native handle by parameter
 * name (`signerHandle` / `coreSignerHandle` / `resolverHandle` /
 * `mnemonicResolverHandle` / `sdkHandle`) must open with a gate bracket
 * (`gate.op {` / `teardownGate.op {` / `queryGate.op {` /
 * `sdk.queryGate.op {`) or visibly delegate to another (gated) method.
 *
 * This exact defect class — a new binding borrowing a handle under plain
 * `withContext(Dispatchers.IO)` — shipped three separate times during
 * review (IdentityRegistration previews, the manager's shielded methods,
 * and the document replace/delete/transfer + contract update bridges), so
 * it is enforced structurally rather than by review vigilance. A method
 * that is intentionally ungated must be listed in [ALLOWLIST] with a
 * reason.
 */
class GateCoverageLintTest {

    @Test
    fun everyHandleBorrowingSuspendFunIsGated() {
        val srcRoot = findSdkMainSources()
        val violations = mutableListOf<String>()

        srcRoot.walkTopDown().filter { it.isFile && it.extension == "kt" }.forEach { file ->
            val src = file.readText()
            var searchFrom = 0
            while (true) {
                val m = FUN_HEADER.find(src, searchFrom) ?: break
                searchFrom = m.range.first + 1
                val name = m.groupValues[1]
                // Walk to the matching close paren of the parameter list.
                var depth = 1
                var i = m.range.last + 1
                while (depth > 0 && i < src.length) {
                    when (src[i]) {
                        '(' -> depth++
                        ')' -> depth--
                    }
                    i++
                }
                val params = src.substring(m.range.last + 1, i - 1)
                if (!HANDLE_PARAM.containsMatchIn(params)) continue
                if ("$name@${file.name}" in ALLOWLIST) continue

                // The opener is whatever follows the (optional) return type.
                val tail = src.substring(i, minOf(i + 260, src.length))
                val gated = GATED_OPENER.containsMatchIn(tail)
                // A delegation body (`= someOtherFun(...)`) is allowed: the
                // delegate is itself scanned.
                val delegates = DELEGATION_OPENER.containsMatchIn(tail) &&
                    !tail.contains("withContext(")
                if (!gated && !delegates) {
                    violations += "${file.relativeTo(srcRoot)}: suspend fun $name " +
                        "borrows a raw handle without a gate bracket"
                }
            }
        }

        assertTrue(
            "Ungated native-handle borrows (wrap in gate.op / queryGate.op, " +
                "or allowlist with a reason):\n" + violations.joinToString("\n"),
            violations.isEmpty(),
        )
    }

    private fun findSdkMainSources(): File {
        // Tests may run with the working dir at the module, project, or repo
        // level — walk up until the SDK main source set is found.
        var dir: File? = File(System.getProperty("user.dir") ?: ".")
        while (dir != null) {
            for (candidate in listOf(
                File(dir, "src/main/kotlin/org/dashfoundation/dashsdk"),
                File(dir, "sdk/src/main/kotlin/org/dashfoundation/dashsdk"),
                File(dir, "packages/kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk"),
            )) {
                if (candidate.isDirectory) return candidate
            }
            dir = dir.parentFile
        }
        error("could not locate the SDK main source set from ${System.getProperty("user.dir")}")
    }

    private companion object {
        val FUN_HEADER = Regex("""suspend fun (\w+)\(""")
        val HANDLE_PARAM = Regex(
            """\b(signerHandle|coreSignerHandle|resolverHandle|mnemonicResolverHandle|sdkHandle)\s*:\s*Long\b""",
        )
        val GATED_OPENER = Regex("""=\s*(\w+\.)?(gate|teardownGate|queryGate)\.op \{""")
        val DELEGATION_OPENER = Regex("""=\s*\w+\(""")

        /** `funName@FileName.kt` entries that are intentionally ungated. */
        val ALLOWLIST = emptySet<String>()
    }
}
