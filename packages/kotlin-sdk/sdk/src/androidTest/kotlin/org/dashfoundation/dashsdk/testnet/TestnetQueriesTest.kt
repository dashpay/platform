package org.dashfoundation.dashsdk.testnet

import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Live testnet read-path checks — opt-in via `-Ptestnet=true` (see
 * [TestnetTest]). Fixtures are the known-good testnet values
 * `DiagnosticsView.swift` uses (`TestData`, themselves shared with the
 * WASM SDK docs) so all three SDKs verify against the same objects.
 *
 * Each test builds a fresh trusted testnet SDK (no wallet, no
 * persistence) and exercises one query family end-to-end: connectivity,
 * proof verification, and JSON marshalling through the JNI shim.
 */
@RunWith(AndroidJUnit4::class)
@TestnetTest
class TestnetQueriesTest {

    @get:Rule
    val testnetGuard = TestnetGuard()

    private fun <T> withTestnetSdk(block: suspend (Sdk) -> T): T = runBlocking {
        Sdk.create(
            SdkConfig(
                network = Network.TESTNET,
                requestRetryCount = 3,
                requestTimeoutMs = 30_000,
            ),
        ).use { sdk -> block(sdk) }
    }

    @Test
    fun identityFetchByKnownTestnetId() {
        val json = withTestnetSdk { sdk ->
            sdk.identities.fetch(KNOWN_IDENTITY_ID)
        }
        assertNotNull("known testnet identity should resolve", json)
        assertTrue(
            "identity JSON should carry an id field",
            json!!.contains("id"),
        )
    }

    @Test
    fun dpnsResolveKnownName() {
        val json = withTestnetSdk { sdk ->
            sdk.dpns.resolve("$KNOWN_DPNS_LABEL.dash")
        }
        assertNotNull("known testnet DPNS name should resolve", json)
    }

    @Test
    fun dataContractFetchDpnsContract() {
        val json = withTestnetSdk { sdk ->
            sdk.contracts.fetchJson(DPNS_CONTRACT_ID)
        }
        assertNotNull("DPNS contract should fetch", json)
        assertTrue(
            "DPNS contract should define the domain document type",
            json!!.contains("domain"),
        )
    }

    private companion object {
        /** ← `DiagnosticsView.swift` `TestData.testIdentityId`. */
        const val KNOWN_IDENTITY_ID = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"

        /** ← `DiagnosticsView.swift` `TestData.dpnsContractId`. */
        const val DPNS_CONTRACT_ID = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"

        /** ← `DiagnosticsView.swift` `TestData.testUsername`. */
        const val KNOWN_DPNS_LABEL = "therealslimshaddy5"
    }
}
