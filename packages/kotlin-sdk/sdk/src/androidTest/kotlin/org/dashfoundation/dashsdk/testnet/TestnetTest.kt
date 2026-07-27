package org.dashfoundation.dashsdk.testnet

import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assume
import org.junit.rules.TestRule
import org.junit.runner.Description
import org.junit.runners.model.Statement

/**
 * Marks a test (or test class) as requiring live testnet connectivity.
 * Tagged tests are skipped unless the run opts in with
 *
 * ```
 * ./gradlew :sdk:connectedDebugAndroidTest -Ptestnet=true
 * ```
 *
 * The Gradle property is forwarded as the `testnet` instrumentation
 * argument (see `sdk/build.gradle.kts`); [TestnetGuard] turns its absence
 * into a JUnit assumption failure (reported as skipped, not failed) so
 * default CI runs stay hermetic.
 */
@Retention(AnnotationRetention.RUNTIME)
@Target(AnnotationTarget.CLASS, AnnotationTarget.FUNCTION)
annotation class TestnetTest

/**
 * JUnit rule enforcing the [TestnetTest] opt-in. Apply alongside the
 * annotation:
 *
 * ```
 * @get:Rule val testnetGuard = TestnetGuard()
 * ```
 */
class TestnetGuard : TestRule {
    override fun apply(base: Statement, description: Description): Statement =
        object : Statement() {
            override fun evaluate() {
                val tagged = description.getAnnotation(TestnetTest::class.java) != null ||
                    description.testClass?.getAnnotation(TestnetTest::class.java) != null
                if (tagged) {
                    val enabled = InstrumentationRegistry.getArguments()
                        .getString("testnet")
                        ?.toBoolean() == true
                    Assume.assumeTrue(
                        "Testnet integration tests are opt-in — run with -Ptestnet=true",
                        enabled,
                    )
                }
                base.evaluate()
            }
        }
}
