package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.persistence.entities.DashpayContactProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressKind
import org.json.JSONArray
import org.json.JSONObject

/**
 * Plain data classes + org.json parsers for the JSON-string reads on the
 * DashPay FFI surface (`Dashpay.getProfile` / `getContactProfile`,
 * `searchDpnsNames`, `PlatformWalletManager.accountBalances`, and
 * `Dashpay.payments`). Field shapes are documented on
 * `org.dashfoundation.dashsdk.ffi.DashpayNative`; parsing happens here,
 * Kotlin-side, following the SDK's JSON-string-read precedent.
 *
 * Parsers are total and lenient: a null/blank/unparseable input yields null
 * (or an empty list), and per-row failures are skipped rather than thrown,
 * so a single malformed row never blanks the whole list.
 */

// ── Profile (getProfile / getContactProfile) ─────────────────────────────

/**
 * Cached DashPay profile — the public fields the requests/contacts UI
 * renders. The payment addresses are in their stored form (lower hex in the
 * JSON); `DashPayPaymentAddresses.string` renders them.
 */
data class DashPayProfile(
    val displayName: String?,
    val publicMessage: String?,
    val avatarUrl: String?,
    val corePaymentAddress: ByteArray? = null,
    val platformPaymentAddress: ByteArray? = null,
    val shieldedAddress: ByteArray? = null,
) {
    /** The published address of [kind] in its stored form, or null. */
    fun paymentAddress(kind: DashPayPaymentAddressKind): ByteArray? = when (kind) {
        DashPayPaymentAddressKind.CORE -> corePaymentAddress
        DashPayPaymentAddressKind.PLATFORM -> platformPaymentAddress
        DashPayPaymentAddressKind.SHIELDED -> shieldedAddress
    }

    override fun equals(other: Any?): Boolean =
        other is DashPayProfile && displayName == other.displayName &&
            publicMessage == other.publicMessage && avatarUrl == other.avatarUrl &&
            corePaymentAddress.contentEquals(other.corePaymentAddress) &&
            platformPaymentAddress.contentEquals(other.platformPaymentAddress) &&
            shieldedAddress.contentEquals(other.shieldedAddress)

    override fun hashCode(): Int {
        var h = displayName.hashCode()
        h = 31 * h + publicMessage.hashCode()
        h = 31 * h + avatarUrl.hashCode()
        h = 31 * h + corePaymentAddress.contentHashCode()
        h = 31 * h + platformPaymentAddress.contentHashCode()
        h = 31 * h + shieldedAddress.contentHashCode()
        return h
    }
}

/**
 * A persisted own profile row as a [DashPayProfile]: the last known profile
 * while Rust's in-memory copy is not loaded yet (an own profile is not
 * restored into Rust at launch; the next profile sync fetches it).
 */
fun DashpayProfileEntity.toDashPayProfile(): DashPayProfile = DashPayProfile(
    displayName = displayName,
    publicMessage = publicMessage,
    avatarUrl = avatarUrl,
    corePaymentAddress = corePaymentAddress,
    platformPaymentAddress = platformPaymentAddress,
    shieldedAddress = shieldedAddress,
)

/** A cached contact profile row as a [DashPayProfile]. */
fun DashpayContactProfileEntity.toDashPayProfile(): DashPayProfile = DashPayProfile(
    displayName = displayName,
    publicMessage = publicMessage,
    avatarUrl = avatarUrl,
    corePaymentAddress = corePaymentAddress,
    platformPaymentAddress = platformPaymentAddress,
    shieldedAddress = shieldedAddress,
)

/** Parse a `getProfile` / `getContactProfile` JSON object, or null. */
fun parseDashPayProfile(json: String?): DashPayProfile? {
    val obj = json?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return null
    return DashPayProfile(
        displayName = obj.optStringOrNull("displayName"),
        publicMessage = obj.optStringOrNull("publicMessage"),
        avatarUrl = obj.optStringOrNull("avatarUrl"),
        corePaymentAddress = obj.optStringOrNull("corePaymentAddress")?.hexOrNull(),
        platformPaymentAddress = obj.optStringOrNull("platformPaymentAddress")?.hexOrNull(),
        shieldedAddress = obj.optStringOrNull("shieldedAddress")?.hexOrNull(),
    )
}

// ── DPNS search (searchDpnsNames) ────────────────────────────────────────

/** One DPNS prefix-search hit: the resolved label + its 32-byte identity id. */
data class DpnsSearchResult(
    /** The DPNS label (Swift `fullName`) — also the row's testTag suffix. */
    val label: String,
    val identityId: ByteArray,
) {
    override fun equals(other: Any?): Boolean =
        other is DpnsSearchResult && label == other.label &&
            identityId.contentEquals(other.identityId)

    override fun hashCode(): Int = 31 * label.hashCode() + identityId.contentHashCode()
}

/** Parse a `searchDpnsNames` JSON array of `{"label":…,"identityId":…hex}`. */
fun parseDpnsSearchResults(json: String?): List<DpnsSearchResult> {
    val array = json?.let { runCatching { JSONArray(it) }.getOrNull() } ?: return emptyList()
    val out = ArrayList<DpnsSearchResult>(array.length())
    for (i in 0 until array.length()) {
        val row = array.optJSONObject(i) ?: continue
        val label = row.optStringOrNull("label") ?: continue
        val id = row.optStringOrNull("identityId")?.hexOrNull()?.takeIf { it.size == 32 } ?: continue
        out.add(DpnsSearchResult(label, id))
    }
    return out
}

// ── Account balances (PlatformWalletManager.accountBalances) ──────────────

/**
 * One per-account balance row. The DashPay "received from contacts" total is
 * the sum of `confirmed + unconfirmed` over rows with [typeTag] == 12
 * (`DashpayReceivingFunds`) whose [userIdentityId] matches the identity.
 */
data class AccountBalance(
    val typeTag: Int,
    val userIdentityId: ByteArray,
    val friendIdentityId: ByteArray,
    val confirmed: Long,
    val unconfirmed: Long,
) {
    override fun equals(other: Any?): Boolean =
        other is AccountBalance && typeTag == other.typeTag &&
            userIdentityId.contentEquals(other.userIdentityId) &&
            friendIdentityId.contentEquals(other.friendIdentityId) &&
            confirmed == other.confirmed && unconfirmed == other.unconfirmed

    override fun hashCode(): Int {
        var result = typeTag
        result = 31 * result + userIdentityId.contentHashCode()
        result = 31 * result + friendIdentityId.contentHashCode()
        result = 31 * result + confirmed.hashCode()
        result = 31 * result + unconfirmed.hashCode()
        return result
    }
}

/** Parse a `accountBalances` JSON array, or an empty list. */
fun parseAccountBalances(json: String?): List<AccountBalance> {
    val array = json?.let { runCatching { JSONArray(it) }.getOrNull() } ?: return emptyList()
    val out = ArrayList<AccountBalance>(array.length())
    for (i in 0 until array.length()) {
        val row = array.optJSONObject(i) ?: continue
        out.add(
            AccountBalance(
                typeTag = row.optInt("typeTag"),
                userIdentityId = row.optStringOrNull("userIdentityId")?.hexOrNull() ?: ByteArray(0),
                friendIdentityId = row.optStringOrNull("friendIdentityId")?.hexOrNull()
                    ?: ByteArray(0),
                confirmed = row.optLong("confirmed"),
                unconfirmed = row.optLong("unconfirmed"),
            ),
        )
    }
    return out
}

// ── Payment history (Dashpay.payments) ───────────────────────────────────

/** One DashPay payment. [direction] 0 Sent / 1 Received; [status] 0 Pending / 1 Confirmed / 2 Failed. */
data class DashPayPayment(
    val txid: String,
    val counterpartyId: ByteArray,
    val amountDuffs: Long,
    val direction: Int,
    val status: Int,
    val memo: String?,
) {
    override fun equals(other: Any?): Boolean =
        other is DashPayPayment && txid == other.txid &&
            counterpartyId.contentEquals(other.counterpartyId) &&
            amountDuffs == other.amountDuffs && direction == other.direction &&
            status == other.status && memo == other.memo

    override fun hashCode(): Int {
        var result = txid.hashCode()
        result = 31 * result + counterpartyId.contentHashCode()
        result = 31 * result + amountDuffs.hashCode()
        result = 31 * result + direction
        result = 31 * result + status
        result = 31 * result + (memo?.hashCode() ?: 0)
        return result
    }
}

/** Parse a `payments` JSON array (rows missing txid/counterparty are skipped). */
fun parseDashPayPayments(json: String?): List<DashPayPayment> {
    val array = json?.let { runCatching { JSONArray(it) }.getOrNull() } ?: return emptyList()
    val out = ArrayList<DashPayPayment>(array.length())
    for (i in 0 until array.length()) {
        val row = array.optJSONObject(i) ?: continue
        val txid = row.optStringOrNull("txid") ?: continue
        val counterparty = row.optStringOrNull("counterpartyId")?.hexOrNull()
            ?.takeIf { it.size == 32 } ?: continue
        out.add(
            DashPayPayment(
                txid = txid,
                counterpartyId = counterparty,
                amountDuffs = row.optLong("amountDuffs"),
                direction = row.optInt("direction"),
                status = row.optInt("status"),
                memo = row.optStringOrNull("memo"),
            ),
        )
    }
    return out
}

// ── Helpers ──────────────────────────────────────────────────────────────

/** `optString` but null (not the JSON literal `"null"` / empty) when absent. */
private fun JSONObject.optStringOrNull(key: String): String? =
    if (isNull(key)) null else optString(key, "").takeIf { it.isNotEmpty() }

/** Lower/upper-hex → bytes; null on odd length or a non-hex digit. */
private fun String.hexOrNull(): ByteArray? {
    if (length % 2 != 0) return null
    return runCatching {
        chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }.getOrNull()
}
