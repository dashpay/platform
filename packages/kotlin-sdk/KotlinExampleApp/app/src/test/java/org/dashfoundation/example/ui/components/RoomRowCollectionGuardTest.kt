package org.dashfoundation.example.ui.components

import java.io.File
import java.lang.reflect.ParameterizedType
import java.lang.reflect.Type
import java.lang.reflect.WildcardType
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.DocumentTypeEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.IndexEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.PropertyEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletManagerMetadataEntity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Every screen that shows rows from a Room flow of an entity that compares
 * equal by its key alone collects them with [collectEachAsStateWithLifecycle].
 *
 * Plain `collectAsStateWithLifecycle` keeps the value it holds when the next
 * one is equal, so a rewritten row (a new balance, a confirmed transaction,
 * a contract update) never reached a screen that was already open. A flow
 * mapped to plain values first, or read once with `.first()`, is unaffected;
 * the mapped ones are listed in [plainValueCollectors].
 */
class RoomRowCollectionGuardTest {

    /**
     * Entities whose `equals` compares the key only (the SwiftData identity
     * the SDK ports). Their ByteArray columns rule out the data-class
     * `equals`, and effects keyed on them rely on it not changing with
     * every column.
     */
    private val keyEqualEntities: Set<Class<*>> = setOf(
        AccountEntity::class.java,
        DataContractEntity::class.java,
        DocumentTypeEntity::class.java,
        IdentityEntity::class.java,
        IndexEntity::class.java,
        PlatformAddressesSyncStateEntity::class.java,
        PropertyEntity::class.java,
        ShieldedNoteEntity::class.java,
        TokenEntity::class.java,
        TransactionEntity::class.java,
        TxoEntity::class.java,
        WalletEntity::class.java,
        WalletManagerMetadataEntity::class.java,
    )

    /**
     * Collectors allowed to stay plain, because they map the rows to values
     * that compare by content before collecting: `file` + `dao.method`.
     */
    private val plainValueCollectors = setOf(
        "ui/credits/TopUpIdentityScreen.kt identityDao.observeByIdentityId", // label, balance, wallet id
        "ui/dashpay/SendShieldedTipScreen.kt identityDao.observeByIdentityId", // wallet id hex
        "ui/shielded/ShieldedActivityScreen.kt identityDao.observeByWallet", // id hex to label
        "ui/shielded/ShieldedTokenSpendScreens.kt identityDao.observeByWallet", // id to label pairs
        "ui/wallet/SendTransactionScreen.kt identityDao.observeByWallet", // IdentitySourceOption values
        "ui/wallet/WalletDetailScreen.kt shieldedDao.observeUnspentNotesByWallet", // has notes
        "ui/wallet/WalletDetailScreen.kt txoDao.observeByWallet", // transaction count
    )

    private val sourceRoot = File("src/main/java/org/dashfoundation/example")

    /** `dao accessor` to the DAO's flow methods that emit [keyEqualEntities]. */
    private fun keyEqualFlows(): Map<String, Set<String>> =
        DashDatabase::class.java.declaredMethods
            .filter { it.parameterCount == 0 && it.returnType.simpleName.endsWith("Dao") }
            .associate { accessor ->
                accessor.name to accessor.returnType.methods
                    .filter { emitsKeyEqualEntity(it.genericReturnType) }
                    .map { it.name }
                    .toSet()
            }
            .filterValues { it.isNotEmpty() }

    private fun emitsKeyEqualEntity(type: Type): Boolean {
        if (type !is ParameterizedType || type.rawType != Flow::class.java) return false
        var element = unwrap(type.actualTypeArguments[0])
        if (element is ParameterizedType && element.rawType == List::class.java) {
            element = unwrap(element.actualTypeArguments[0])
        }
        return element in keyEqualEntities
    }

    private fun unwrap(type: Type): Type =
        if (type is WildcardType) type.upperBounds.first() else type

    @Test
    fun `should find the key-equal flows the screens read`() {
        val flows = keyEqualFlows()
        assertTrue(flows["identityDao"].orEmpty().contains("observeByIdentityId"))
        assertTrue(flows["tokenDao"].orEmpty().contains("observeTokenById"))
        assertTrue(flows["transactionDao"].orEmpty().contains("observeByTxid"))
        assertTrue(sourceRoot.isDirectory)
    }

    @Test
    fun `should collect every key-equal Room flow so a rewritten row reaches the screen`() {
        val flows = keyEqualFlows()
        val call = Regex("""(\w+Dao)\(\)\s*\.\s*(\w+)\(""")
        val consumer = Regex(
            """\.first\(\)|collectEachAsStateWithLifecycle\(|collectAsStateWithLifecycle\(|collectAsState\(""",
        )
        val violations = mutableListOf<String>()
        sourceRoot.walkTopDown().filter { it.extension == "kt" }.forEach { file ->
            val source = file.readText()
            val relative = file.relativeTo(sourceRoot).invariantSeparatorsPath
            call.findAll(source).forEach { match ->
                val (dao, method) = match.destructured
                if (method !in flows[dao].orEmpty()) return@forEach
                // The first consumer within the statement (a few lines on).
                val tail = source.substring(match.range.last).lineSequence().take(15).joinToString("\n")
                val used = consumer.find(tail)?.value ?: return@forEach
                val plain = used.startsWith("collectAsState")
                if (plain && "$relative $dao.$method" !in plainValueCollectors) {
                    val line = source.substring(0, match.range.first).count { it == '\n' } + 1
                    violations += "$relative:$line $dao.$method"
                }
            }
        }
        assertEquals(
            "Collect these with collectEachAsStateWithLifecycle, or list a collector " +
                "that maps the rows to plain values in plainValueCollectors",
            emptyList<String>(),
            violations,
        )
    }
}
