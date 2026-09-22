package org.dashfoundation.dashsdk.errors

/**
 * The consensus error Platform rejected a state transition with, as the
 * native layer reported it: the numeric [code] rs-dpp assigns the error
 * (`packages/rs-dpp/src/errors/consensus/codes.rs`) and the [kind] it belongs
 * to. Branch on these rather than on the error message, whose wording is not
 * a contract.
 *
 * Read it from [DashSdkError.consensusError]. It is present for the wallet
 * operations whose native error still holds the SDK's consensus verdict (the
 * token state transitions among them) and `null` for every failure that was
 * not a consensus rejection.
 */
data class PlatformConsensusError(
    val code: Int,
    val kind: ConsensusErrorKind,
)

/**
 * Which of rs-dpp's consensus error families an error belongs to. The native
 * layer reports it next to the code, so it is never derived from the number
 * on this side. Values mirror `PlatformWalletFFIConsensusErrorKind` in
 * `rs-platform-wallet-ffi/src/error.rs`.
 */
enum class ConsensusErrorKind {
    /** Structure or version validation failed; the transition was not executed. */
    BASIC,

    /** The transition's signature or signing key was rejected. */
    SIGNATURE,

    /** The fee could not be covered. */
    FEE,

    /** The transition was well-formed but Platform state rejected it. */
    STATE,

    /** A family this SDK build does not know. The [PlatformConsensusError.code] still holds. */
    UNKNOWN,
    ;

    internal companion object {
        fun fromNative(value: Int): ConsensusErrorKind = when (value) {
            1 -> BASIC
            2 -> SIGNATURE
            3 -> FEE
            4 -> STATE
            else -> UNKNOWN
        }
    }
}
