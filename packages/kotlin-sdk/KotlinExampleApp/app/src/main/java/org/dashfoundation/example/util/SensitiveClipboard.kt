package org.dashfoundation.example.util

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.PersistableBundle
import android.os.SystemClock
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Copies a secret to the clipboard and clears it about 60 s later, the
 * Android counterpart of the 60 s `UIPasteboard` expiry in
 * `CoreAddressDetailView` (`StorageRecordDetailViews.swift`). Android has no
 * clipboard expiry, so the app clears the clip itself.
 *
 * Best effort: from Android 10 an app that is not focused cannot read the
 * clipboard, so it cannot tell whether its copy is still there. A clear that
 * falls due while the app is unfocused is kept pending and retried when a
 * window of the app gains focus again ([MainActivity][org.dashfoundation.example.MainActivity]
 * calls [onFocusGained]). The clip is cleared only while it still carries this
 * copy's label, so a newer copy by anyone survives.
 */
internal object SensitiveClipboard {

    /** `ClipDescription.EXTRA_IS_SENSITIVE`; a keyboard hint on API 32 and lower. */
    private const val EXTRA_IS_SENSITIVE = "android.content.extra.IS_SENSITIVE"
    private const val EXPIRY_MS = 60_000L

    private class Pending(val label: String, val dueAtElapsedMs: Long)

    @Volatile
    private var pending: Pending? = null

    /**
     * Copy [value] marked sensitive and schedule its clear on [scope], which
     * must outlive the calling screen (the application scope).
     */
    fun copy(context: Context, scope: CoroutineScope, value: CharArray) {
        val label = "private-key-${SystemClock.elapsedRealtimeNanos()}"
        val clip = ClipData.newPlainText(label, String(value))
        clip.description.extras = PersistableBundle().apply {
            putBoolean(EXTRA_IS_SENSITIVE, true)
        }
        clipboard(context).setPrimaryClip(clip)
        pending = Pending(label, SystemClock.elapsedRealtime() + EXPIRY_MS)
        val appContext = context.applicationContext
        scope.launch {
            delay(EXPIRY_MS)
            clearIfDue(appContext, focused = false)
        }
    }

    /** A window of the app gained focus: run a clear that fell due meanwhile. */
    fun onFocusGained(context: Context) = clearIfDue(context, focused = true)

    private fun clearIfDue(context: Context, focused: Boolean) {
        val due = pending ?: return
        if (SystemClock.elapsedRealtime() < due.dueAtElapsedMs) return
        val manager = clipboard(context)
        val description = manager.primaryClipDescription
        if (description == null) {
            // Unfocused, the clipboard is unreadable: retry on focus. Focused,
            // it is empty: nothing left to clear.
            if (focused) pending = null
            return
        }
        if (description.label == due.label) manager.clearPrimaryClip()
        pending = null
    }

    private fun clipboard(context: Context): ClipboardManager =
        context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
}
