package com.dash.sdk.utils

import java.util.Base64

/**
 * Extension functions for common conversions
 */

/**
 * Convert ByteArray to hex string
 */
fun ByteArray.toHexString(): String = joinToString("") { "%02x".format(it) }

/**
 * Convert hex string to ByteArray
 */
fun String.hexToByteArray(): ByteArray {
    require(length % 2 == 0) { "Hex string must have even length" }
    return chunked(2)
        .map { it.toInt(16).toByte() }
        .toByteArray()
}

/**
 * Convert ByteArray to base64 string
 */
fun ByteArray.toBase64(): String = Base64.getEncoder().encodeToString(this)

/**
 * Convert base64 string to ByteArray
 */
fun String.base64ToByteArray(): ByteArray = Base64.getDecoder().decode(this)

/**
 * Convert ByteArray to base58 string
 */
fun ByteArray.toBase58(): String {
    // Base58 alphabet (Bitcoin/Dash alphabet)
    val alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    
    // Count leading zeros
    var leadingZeros = 0
    for (byte in this) {
        if (byte == 0.toByte()) {
            leadingZeros++
        } else {
            break
        }
    }
    
    // Convert to big integer
    var num = java.math.BigInteger(1, this)
    val base = java.math.BigInteger.valueOf(58)
    val sb = StringBuilder()
    
    while (num > java.math.BigInteger.ZERO) {
        val remainder = num.mod(base)
        sb.append(alphabet[remainder.toInt()])
        num = num.divide(base)
    }
    
    // Add leading 1s for zeros
    repeat(leadingZeros) {
        sb.append('1')
    }
    
    return sb.reverse().toString()
}

/**
 * Convert base58 string to ByteArray
 */
fun String.base58ToByteArray(): ByteArray {
    val alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    val base = java.math.BigInteger.valueOf(58)
    
    // Count leading 1s
    var leadingOnes = 0
    for (char in this) {
        if (char == '1') {
            leadingOnes++
        } else {
            break
        }
    }
    
    var num = java.math.BigInteger.ZERO
    for (char in this) {
        val digit = alphabet.indexOf(char)
        require(digit >= 0) { "Invalid base58 character: $char" }
        num = num.multiply(base).add(java.math.BigInteger.valueOf(digit.toLong()))
    }
    
    val bytes = num.toByteArray()
    
    // Remove sign byte if present
    val result = if (bytes[0] == 0.toByte() && bytes.size > 1) {
        bytes.sliceArray(1 until bytes.size)
    } else {
        bytes
    }
    
    // Add leading zeros
    return ByteArray(leadingOnes) + result
}

/**
 * Ensure ByteArray is exactly the specified size
 */
fun ByteArray.ensureSize(size: Int): ByteArray {
    return when {
        this.size == size -> this
        this.size < size -> ByteArray(size - this.size) + this
        else -> this.takeLast(size).toByteArray()
    }
}