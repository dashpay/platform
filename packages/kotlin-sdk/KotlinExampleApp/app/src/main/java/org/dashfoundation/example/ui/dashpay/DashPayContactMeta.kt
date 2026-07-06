package org.dashfoundation.example.ui.dashpay

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.SubcomposeAsyncImage
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.example.util.toHex

/**
 * Device-local, per-contact metadata for the DashPay tab — port of
 * `DashPayContactMeta.swift`'s `DashPayContactMetaStore`: alias, note,
 * hidden flag, and a DPNS-label hint captured at add time.
 *
 * These are scoped to "This device only" — a later milestone replaces
 * this store with `contactInfo` documents synced via Platform. Until
 * then [SharedPreferences] is the honest backing (no sync semantics), the
 * counterpart of iOS `UserDefaults`.
 *
 * Keys are scoped by `(network, owner identity, contact identity)` so two
 * owner identities (or two networks) never share a contact's alias. The
 * [version] counter is bumped on every write so Compose readers that
 * compute reads through this store recompose after a write — plain
 * SharedPreferences reads don't participate in Compose invalidation.
 */
class DashPayContactMetaStore(context: Context) {

    private val prefs = context.getSharedPreferences("dashpay_contact_meta", Context.MODE_PRIVATE)

    /** Bumped on every write so observing composables recompute reads. */
    private val _version = MutableStateFlow(0)
    val version: StateFlow<Int> = _version.asStateFlow()

    // ── Alias (local display-name override) ──────────────────────────────

    fun alias(network: Network, owner: ByteArray, contact: ByteArray): String? =
        nonEmpty(prefs.getString(key("alias", network, owner, contact), null))

    fun setAlias(alias: String?, network: Network, owner: ByteArray, contact: ByteArray) {
        write(nonEmpty(alias), key("alias", network, owner, contact))
    }

    // ── Note ─────────────────────────────────────────────────────────────

    fun note(network: Network, owner: ByteArray, contact: ByteArray): String? =
        nonEmpty(prefs.getString(key("note", network, owner, contact), null))

    fun setNote(note: String?, network: Network, owner: ByteArray, contact: ByteArray) {
        write(nonEmpty(note), key("note", network, owner, contact))
    }

    // ── Hidden ───────────────────────────────────────────────────────────

    fun isHidden(network: Network, owner: ByteArray, contact: ByteArray): Boolean =
        prefs.getBoolean(key("hidden", network, owner, contact), false)

    fun setHidden(hidden: Boolean, network: Network, owner: ByteArray, contact: ByteArray) {
        prefs.edit().putBoolean(key("hidden", network, owner, contact), hidden).apply()
        _version.value += 1
    }

    // ── DPNS hint ────────────────────────────────────────────────────────

    /**
     * DPNS label observed when the contact was added via username search.
     * Display-precedence fallback only — contacts' DPNS labels aren't
     * persisted in Room (only managed identities' are), so this hint is
     * "the data available" for the contact rows.
     */
    fun dpnsHint(network: Network, owner: ByteArray, contact: ByteArray): String? =
        nonEmpty(prefs.getString(key("dpnsHint", network, owner, contact), null))

    fun setDpnsHint(name: String?, network: Network, owner: ByteArray, contact: ByteArray) {
        write(nonEmpty(name), key("dpnsHint", network, owner, contact))
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    private fun key(field: String, network: Network, owner: ByteArray, contact: ByteArray): String =
        "dashpay.meta.$field.${network.ffiValue}.${owner.toHex()}.${contact.toHex()}"

    private fun write(value: String?, key: String) {
        prefs.edit().apply {
            if (value != null) putString(key, value) else remove(key)
        }.apply()
        _version.value += 1
    }

    private fun nonEmpty(value: String?): String? = value?.trim()?.takeIf { it.isNotEmpty() }
}

// ── Display-name precedence ──────────────────────────────────────────────

/**
 * Resolve the display precedence for a DashPay contact — port of
 * `dashPayContactDisplayName` in `DashPayContactMeta.swift`: local alias →
 * DashPay profile `displayName` → DPNS label → truncated hex id. Every
 * input but the id is optional; blank/whitespace strings count as absent.
 */
fun dashPayContactDisplayName(
    contactId: ByteArray,
    alias: String?,
    profileDisplayName: String?,
    dpnsLabel: String?,
): String {
    for (candidate in listOf(alias, profileDisplayName, dpnsLabel)) {
        val trimmed = candidate?.trim()
        if (!trimmed.isNullOrEmpty()) return trimmed
    }
    return contactId.toHex().take(12) + "…"
}

// ── Txid display order ───────────────────────────────────────────────────

/**
 * Hex-encode a raw 32-byte txid in canonical (reversed) display order —
 * port of `txidDisplayHex` in `DashPayContactMeta.swift`. The FFI hands back
 * wire/internal byte order, so a bare hex reads reversed from block
 * explorers; this flip lines it up with the rest of the app's tx display.
 */
fun txidDisplayHex(txid: ByteArray): String =
    txid.reversed().joinToString("") { "%02x".format(it) }

// ── Avatar ───────────────────────────────────────────────────────────────

/**
 * Shared avatar bubble — port of `DashPayAvatarView`: the profile's
 * `avatarUrl` loaded via Coil when present, an initial-circle fallback
 * otherwise. The initial comes from the resolved display name; the tint is
 * fixed (the same for every contact, not name-hashed), theme-aware via
 * [MaterialTheme].
 */
@Composable
fun DashPayAvatar(avatarUrl: String?, displayName: String, size: Dp = 40.dp) {
    val url = avatarUrl?.trim()?.takeIf { it.isNotEmpty() }
    if (url != null) {
        SubcomposeAsyncImage(
            model = url,
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.size(size).clip(CircleShape),
            loading = { InitialsCircle(displayName, size) },
            error = { InitialsCircle(displayName, size) },
        )
    } else {
        InitialsCircle(displayName, size)
    }
}

@Composable
private fun InitialsCircle(displayName: String, size: Dp) {
    Box(
        modifier = Modifier
            .size(size)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.2f)),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = displayName.take(1).uppercase(),
            color = MaterialTheme.colorScheme.primary,
            style = if (size > 50.dp) {
                MaterialTheme.typography.titleLarge
            } else {
                MaterialTheme.typography.titleMedium
            },
        )
    }
}
