package org.dashfoundation.example.util

import android.graphics.Bitmap
import android.graphics.Color
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter

/**
 * Render [content] as a QR bitmap via zxing — the Android counterpart of
 * `ReceiveAddressView.generateQRCode(from:)` (CoreImage `CIQRCodeGenerator`).
 * Returns null when encoding fails (empty/oversized content).
 */
fun generateQrBitmap(content: String, sizePx: Int = 512): Bitmap? = try {
    val matrix = QRCodeWriter().encode(
        content,
        BarcodeFormat.QR_CODE,
        sizePx,
        sizePx,
        mapOf(EncodeHintType.MARGIN to 1),
    )
    val pixels = IntArray(sizePx * sizePx) { i ->
        if (matrix.get(i % sizePx, i / sizePx)) Color.BLACK else Color.WHITE
    }
    Bitmap.createBitmap(pixels, sizePx, sizePx, Bitmap.Config.RGB_565)
} catch (_: Exception) {
    null
}

/**
 * The meaningful result of decoding a scanned/pasted payment string —
 * port of `ScannedPayment` + `QRPayloadParser.parse` (QRScannerView.swift).
 *
 * Accepts a bare address, `dash:ADDRESS`, `dash://ADDRESS`, and
 * `dash:ADDRESS?amount=1.23&label=x`. The `dash:` scheme match is
 * case-insensitive but the address casing is preserved (base58check /
 * bech32m payloads are case-sensitive).
 *
 * Address validation is intentionally lighter than iOS (which routes
 * through the Rust `DashAddress.parse` FFI — not bridged in the Kotlin
 * SDK yet): any non-empty candidate without whitespace is accepted.
 */
data class ScannedPayment(val address: String, val amount: String?) {
    companion object {
        fun parse(raw: String): ScannedPayment? {
            val trimmed = raw.trim()
            if (trimmed.isEmpty()) return null

            var remainder = trimmed
            val scheme = "dash:"
            if (remainder.length >= scheme.length &&
                remainder.substring(0, scheme.length).lowercase() == scheme
            ) {
                remainder = remainder.substring(scheme.length).removePrefix("//")
            }

            val parts = remainder.split("?", limit = 2)
            val candidate = parts[0].trim()
            if (candidate.isEmpty() || candidate.any { it.isWhitespace() }) return null

            val amount = parts.getOrNull(1)?.let(::positiveAmount)
            return ScannedPayment(candidate, amount)
        }

        /** Extract a strictly-positive `amount=` value from a query string. */
        private fun positiveAmount(query: String): String? {
            for (pair in query.split("&")) {
                val kv = pair.split("=", limit = 2)
                if (kv.size != 2 || kv[0].lowercase() != "amount") continue
                val value = java.net.URLDecoder.decode(kv[1], "UTF-8")
                return value.takeIf { (it.toDoubleOrNull() ?: 0.0) > 0 }
            }
            return null
        }
    }
}
