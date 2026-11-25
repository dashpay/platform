package org.dashfoundation.example.ui.wallet

import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.example.ui.dashpay.parseAccountBalances
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The balance a wallet account row shows. Runs on Robolectric for Android's
 * own org.json, which parses the JNI per-account balance rows.
 */
@RunWith(RobolectricTestRunner::class)
class AccountRowBalanceTest {

    private val walletId = ByteArray(32) { 1 }

    private fun account(id: Long, type: Int, index: Int, standardTag: Int = 0) = AccountEntity(
        id = id,
        walletId = walletId,
        accountType = type,
        accountIndex = index,
        accountTypeName = "account",
        standardTag = standardTag,
    )

    // The JNI row shape (`DashpayNative.walletManagerAccountBalances`).
    private val rustRows = parseAccountBalances(
        """[
          {"typeTag":0,"standardTag":0,"index":0,"registrationIndex":0,"keyClass":0,
           "userIdentityId":"${"00".repeat(32)}","friendIdentityId":"${"00".repeat(32)}",
           "confirmed":1000000000,"unconfirmed":5000,"immature":0,"locked":0,
           "keysUsed":3,"keysTotal":30},
          {"typeTag":0,"standardTag":0,"index":1,"registrationIndex":0,"keyClass":0,
           "userIdentityId":"${"00".repeat(32)}","friendIdentityId":"${"00".repeat(32)}",
           "confirmed":7,"unconfirmed":0,"immature":0,"locked":0,"keysUsed":0,"keysTotal":30},
          {"typeTag":0,"standardTag":1,"index":0,"registrationIndex":0,"keyClass":0,
           "userIdentityId":"${"00".repeat(32)}","friendIdentityId":"${"00".repeat(32)}",
           "confirmed":42,"unconfirmed":0,"immature":0,"locked":0,"keysUsed":0,"keysTotal":30}
        ]""",
    )

    @Test
    fun `should show the Core balance Rust reports for the account row`() {
        // The QA wallet: Core Balance 10 DASH, the BIP44 #0 row read 0 DASH.
        assertEquals(
            AccountRowBalance.Core(confirmed = 1_000_000_000, unconfirmed = 5_000),
            accountRowBalance(account(1, type = 0, index = 0), rustRows),
        )
    }

    @Test
    fun `should match the Rust row on type tag, standard tag and index`() {
        assertEquals(
            AccountRowBalance.Core(confirmed = 7, unconfirmed = 0),
            accountRowBalance(account(2, type = 0, index = 1), rustRows),
        )
        assertEquals(
            AccountRowBalance.Core(confirmed = 42, unconfirmed = 0),
            accountRowBalance(account(3, type = 0, index = 0, standardTag = 1), rustRows),
        )
    }

    @Test
    fun `should show an unread Core balance as unknown, never as zero`() {
        // Nothing read yet (manager starting or replaced).
        assertEquals(
            AccountRowBalance.CoreUnknown,
            accountRowBalance(account(1, type = 0, index = 0), null),
        )
        // A read with no row for this account (wallet not loaded in Rust).
        assertEquals(
            AccountRowBalance.CoreUnknown,
            accountRowBalance(account(4, type = 0, index = 9), rustRows),
        )
        assertEquals(
            AccountRowBalance.CoreUnknown,
            accountRowBalance(account(1, type = 0, index = 0), emptyList()),
        )
    }

    @Test
    fun `should show no PlatformPayment balance until Rust offers an account total`() {
        assertEquals(
            AccountRowBalance.PlatformNotShown,
            accountRowBalance(account(5, type = 14, index = 0), rustRows),
        )
    }

    @Test
    fun `should show no balance for a special purpose account`() {
        assertEquals(
            AccountRowBalance.None,
            accountRowBalance(account(7, type = 3, index = 0), rustRows),
        )
    }
}
