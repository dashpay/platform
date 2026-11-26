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

    private val consumer = Regex(
        """\.first\(\)|collectEachAsStateWithLifecycle\(|collectAsStateWithLifecycle\(|collectAsState\(""",
    )

    /**
     * The problems in one source file: a key-equal flow collected plainly, a
     * collected flow built outside `remember` (a new cold flow, so a new
     * subscription and a new emission, on every recomposition), and a
     * `remember` keyed on a collected value itself (compared by id, so what
     * it derived stays old). `relative` names the file in the messages and in
     * [plainValueCollectors].
     */
    private fun problems(source: String, relative: String, flows: Map<String, Set<String>>): List<String> {
        val lines = source.lines()
        val found = mutableListOf<String>()
        // `val dao = container.database.identityDao()` and then `dao.observeX(`.
        val aliases = Regex("""val (\w+) = [\w.]*?\b(\w+Dao)\(\)\s*$""", RegexOption.MULTILINE)
            .findAll(source).associate { it.groupValues[1] to it.groupValues[2] }
        // Direct `xDao().method(` calls, then calls on an alias of a DAO.
        val direct = Regex("""\b(\w+Dao)\(\)\s*\.\s*(\w+)\(""").findAll(source)
            .map { Triple(it, it.groupValues[1], it.groupValues[2]) }
        val aliased = Regex("""\b(\w+)\s*\.\s*(\w+)\(""").findAll(source)
            .mapNotNull { m -> aliases[m.groupValues[1]]?.let { Triple(m, it, m.groupValues[2]) } }
        (direct + aliased).forEach { (match, dao, method) ->
            if (method !in flows[dao].orEmpty()) return@forEach
            val line = source.substring(0, match.range.first).count { it == '\n' }
            val tail = source.substring(match.range.last).lineSequence().take(15).joinToString("\n")
            val used = consumer.find(tail)?.value ?: return@forEach
            when {
                used.startsWith("collectAsState") ->
                    if ("$relative $dao.$method" !in plainValueCollectors) {
                        found += "$relative:${line + 1} $dao.$method collected plainly"
                    }
                used.startsWith("collectEach") -> {
                    val start = (line downTo 0).firstOrNull {
                        Regex("""^\s*val \w+ (by |= remember)""").containsMatchIn(lines[it])
                    } ?: line
                    if ((start..line).none { "remember" in lines[it] }) {
                        found += "$relative:${line + 1} $dao.$method flow not remembered"
                    }
                }
            }
        }
        // Values collected with collectEachAsStateWithLifecycle, then a
        // remember keyed on one of them as is.
        val collected = lines.indices.filter { "collectEachAsStateWithLifecycle(" in lines[it] }
            .mapNotNull { at ->
                (at downTo maxOf(0, at - 30)).firstNotNullOfOrNull {
                    Regex("""^\s*val (\w+) by """).find(lines[it])?.groupValues?.get(1)
                }
            }
            .toSet()
        lines.forEachIndexed { index, text ->
            Regex("""\bremember\(([^(){}]*)\)""").findAll(text).forEach { key ->
                val bare = key.groupValues[1].split(',').map { it.trim() }.filter { it in collected }
                if (bare.isNotEmpty()) {
                    found += "$relative:${index + 1} remember keyed on $bare (use EmissionKey)"
                }
            }
        }
        return found.sortedBy { it.substringAfter(':').substringBefore(' ').toInt() }
    }

    @Test
    fun `should collect every key-equal Room flow so a rewritten row reaches the screen`() {
        val flows = keyEqualFlows()
        val violations = sourceRoot.walkTopDown().filter { it.extension == "kt" }.flatMap { file ->
            problems(file.readText(), file.relativeTo(sourceRoot).invariantSeparatorsPath, flows)
        }.toList()
        assertEquals(
            "Collect these with collectEachAsStateWithLifecycle from a remembered flow, key " +
                "derived values on EmissionKey, or list a collector that maps the rows to " +
                "plain values in plainValueCollectors",
            emptyList<String>(),
            violations,
        )
    }

    @Test
    fun `should catch an aliased DAO, an inline flow and a remember keyed on the rows`() {
        val flows = keyEqualFlows()
        val source = """
            fun Screen() {
                val picked = remember(networkRaw) {
                    val dao = container.database.identityDao()
                    dao.observeWalletOwned()
                }.collectAsStateWithLifecycle(initialValue = emptyList())
                val identity by container.database.identityDao()
                    .observeByIdentityId(idBytes)
                    .collectEachAsStateWithLifecycle(initialValue = null)
                val tokens by remember(network) {
                    container.database.tokenDao().observeTokensByNetwork(network)
                }.collectEachAsStateWithLifecycle(initialValue = emptyList())
                val sorted = remember(tokens) { tokens.sortedBy { it.name } }
                val fine = remember(EmissionKey(tokens)) { tokens.size }
            }
        """.trimIndent()

        assertEquals(
            listOf(
                "Screen.kt:4 identityDao.observeWalletOwned collected plainly",
                "Screen.kt:6 identityDao.observeByIdentityId flow not remembered",
                "Screen.kt:12 remember keyed on [tokens] (use EmissionKey)",
            ),
            problems(source, "Screen.kt", flows),
        )
    }
}
