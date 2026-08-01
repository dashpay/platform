package org.dashfoundation.dashsdk.wallet

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Source-scanning regression guard for the teardown-fence contract: every
 * `suspend fun` in the SDK that borrows a raw native handle by parameter
 * name (`signerHandle` / `coreSignerHandle` / `resolverHandle` /
 * `mnemonicResolverHandle` / `sdkHandle`) must open with a gate bracket
 * (`gate.op {` / `gate.opWithCleanupOnCancellation(...) {` /
 * `teardownGate.op {` / `queryGate.op {` / `sdk.queryGate.op {`) or visibly
 * delegate to another (gated) method. A block-bodied method may run argument
 * validation and a `withContext(NonCancellable)` delivery wrapper first, but
 * the bracket must still open before the first JNI call.
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

                if (!isGated(src, i)) {
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

    /**
     * @param src the whole file.
     * @param afterParams index just past the parameter list's close paren —
     *   i.e. the start of the optional return type.
     */
    private fun isGated(src: String, afterParams: Int): Boolean {
        // The opener is whatever follows the (optional) return type: `=` for an
        // expression body, `{` for a block body. A declaration with no body at
        // all (none exist today) finds no opener and is reported, so a future
        // abstract handle-borrowing member fails closed into review.
        val openerAt = (afterParams until minOf(afterParams + 260, src.length))
            .firstOrNull { src[it] == '=' || src[it] == '{' }
            ?: return false

        if (src[openerAt] == '=') {
            val tail = src.substring(afterParams, minOf(afterParams + 260, src.length))
            // A delegation body (`= someOtherFun(...)`) is allowed: the
            // delegate is itself scanned.
            val delegates = DELEGATION_OPENER.containsMatchIn(tail) &&
                !tail.contains("withContext(")
            return GATED_OPENER.containsMatchIn(tail) || delegates
        }

        // Block body. The gate bracket cannot be the first token here, because
        // `require(...)` argument validation and a `withContext(NonCancellable)`
        // result-delivery wrapper legitimately precede it — see
        // `IdentityRegistration.createInvitation`, where letting `withContext`
        // discard the gated result would lose the only copy of a bearer
        // credential after the voucher has already been funded. What the
        // teardown fence actually requires is that nothing touches the borrowed
        // handle OUTSIDE the bracket, so demand that the gate opens before the
        // first JNI call rather than at the first statement. A bracket that
        // opens after a native call, or no bracket at all, is still a violation.
        val body = src.substring(openerAt, endOfBlock(src, openerAt))
        val gateAt = GATE_BRACKET.find(body)?.range?.first ?: -1
        val nativeAt = NATIVE_CALL.find(body)?.range?.first ?: -1
        if (gateAt >= 0) return nativeAt < 0 || gateAt < nativeAt
        // No gate and no JNI call: a plain delegation to another (scanned)
        // method, allowed only if it does not dispatch on its own.
        return nativeAt < 0 && !body.contains("withContext(")
    }

    /**
     * Index just past the `}` matching the `{` at [open]. String-interpolation
     * `${...}` braces are balanced, so counting them is harmless.
     */
    private fun endOfBlock(src: String, open: Int): Int {
        var depth = 0
        var i = open
        while (i < src.length) {
            when (src[i]) {
                '{' -> depth++
                '}' -> {
                    depth--
                    if (depth == 0) return i + 1
                }
            }
            i++
        }
        return src.length
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

        /** The gate bracket itself, wherever in a body it appears. */
        const val GATE_BRACKET_PATTERN =
            """(\w+\.)?(gate|teardownGate|queryGate)\.""" +
                """(?:op \{|opWithCleanupOnCancellation\()"""
        val GATE_BRACKET = Regex(GATE_BRACKET_PATTERN)

        /** The bracket in expression-body position, i.e. as the whole body. */
        val GATED_OPENER = Regex("""=\s*$GATE_BRACKET_PATTERN""")
        val DELEGATION_OPENER = Regex("""=\s*\w+\(""")

        /**
         * A JNI entry point — the call that actually consumes the borrowed
         * handle, and so the thing that must sit inside the bracket.
         */
        val NATIVE_CALL = Regex("""\b\w*Native\.\w+\(""")

        /** `funName@FileName.kt` entries that are intentionally ungated. */
        val ALLOWLIST = emptySet<String>()
    }
}
