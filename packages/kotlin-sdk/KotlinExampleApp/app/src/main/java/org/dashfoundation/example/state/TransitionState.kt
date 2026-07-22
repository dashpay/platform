package org.dashfoundation.example.state

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Ephemeral pricing / purchase-eligibility for the state-transition catalog
 * — 1:1 port of `TransitionState.swift`.
 *
 * The `documentPurchase` flow fetches the on-chain document price
 * asynchronously; the detail form's submit button stays disabled until
 * [canPurchaseDocument] flips true and [documentPrice] is populated. All
 * three fields are cleared by [reset] when a fresh transition detail form
 * initializes (the Swift `clearForm()` → `transitionState.reset()` path).
 */
class TransitionState {
    private val _documentPrice = MutableStateFlow<Long?>(null)

    /** The fetched purchase price in credits, or null if unfetched. */
    val documentPrice: StateFlow<Long?> = _documentPrice.asStateFlow()

    private val _canPurchaseDocument = MutableStateFlow(false)

    /** Whether the fetched document is currently purchasable. */
    val canPurchaseDocument: StateFlow<Boolean> = _canPurchaseDocument.asStateFlow()

    private val _documentPurchaseError = MutableStateFlow<String?>(null)

    /** Inline error from the last price fetch, or null. */
    val documentPurchaseError: StateFlow<String?> = _documentPurchaseError.asStateFlow()

    /** Publish a successful price fetch. */
    fun setPrice(priceCredits: Long, purchasable: Boolean) {
        _documentPrice.value = priceCredits
        _canPurchaseDocument.value = purchasable
        _documentPurchaseError.value = null
    }

    /** Publish a failed price fetch. */
    fun setPurchaseError(message: String) {
        _documentPrice.value = null
        _canPurchaseDocument.value = false
        _documentPurchaseError.value = message
    }

    /** Clear all fields — called on fresh transition-detail form init. ← Swift `reset()`. */
    fun reset() {
        _documentPrice.value = null
        _canPurchaseDocument.value = false
        _documentPurchaseError.value = null
    }
}
