package org.dashfoundation.example.services.transitions

/**
 * The state-transition catalog — 1:1 port of `TransitionTypes.swift` +
 * `StateTransitionDefinitions.swift`. Drives the transitions catalog UI:
 * [TransitionCategory] list → per-category transition list → dynamic input
 * form.
 *
 * Execution is wired only for transitions whose underlying FFI is bridged
 * in this milestone ([TransitionDefinition.executable]); the rest surface
 * the named-missing-export dialog (the `SendTransactionScreen` pattern) so
 * the catalog is fully navigable without faking success.
 */

/** The six catalog categories, in iOS order. ← Swift `TransitionCategory`. */
enum class TransitionCategory(
    val displayName: String,
    val description: String,
) {
    ADDRESS("Address", "Transfer and withdraw credits using Platform addresses"),
    IDENTITY("Identity", "Create, update, and manage identities"),
    DATA_CONTRACT("Data Contract", "Deploy and update data contracts"),
    DOCUMENT("Document", "Create and manage documents"),
    TOKEN("Token", "Mint, transfer, and manage tokens"),
    VOTING("Voting", "Participate in governance voting"),
}

/** The rendering kind of a single transition input. ← Swift input `type` strings. */
enum class TransitionInputType {
    TEXT,
    TEXTAREA,
    NUMBER,
    CHECKBOX,
    SELECT,
    JSON,
    IDENTITY_PICKER,
    CONTRACT_PICKER,
    DOCUMENT_TYPE_PICKER,
    DOCUMENT_PICKER,
    DOCUMENT_WITH_PRICE,
    TOKEN_PICKER,
}

/** One selectable option for a [TransitionInputType.SELECT] input. ← Swift `SelectOption`. */
data class SelectOption(val value: String, val label: String)

/** One input field of a transition form. ← Swift `TransitionInput`. */
data class TransitionInput(
    val name: String,
    val type: TransitionInputType,
    val label: String,
    val required: Boolean,
    val placeholder: String? = null,
    val help: String? = null,
    val defaultValue: String? = null,
    val options: List<SelectOption> = emptyList(),
)

/** One transition definition. ← Swift `TransitionDefinition`. */
data class TransitionDefinition(
    val key: String,
    val category: TransitionCategory,
    val label: String,
    val description: String,
    val inputs: List<TransitionInput>,
    /**
     * Whether this milestone can execute the transition through a bridged
     * FFI path. `false` transitions render the form and surface the
     * named-missing-export dialog on submit.
     */
    val executable: Boolean = false,
    /**
     * Where an executable transition routes for its dedicated screen (the
     * Address / credits flows have specialized screens rather than the
     * generic builder). Null → executes inline via the generic detail form.
     */
    val dedicatedRoute: DedicatedTransition? = null,
)

/** Executable transitions with a dedicated (non-generic-builder) screen. */
enum class DedicatedTransition { TRANSFER_CREDITS, WITHDRAW_CREDITS, TOP_UP_IDENTITY }

/** The full catalog, keyed by transition key. ← `TransitionDefinitions.all`. */
object StateTransitionDefinitions {

    private val select = TransitionInputType.SELECT
    private val text = TransitionInputType.TEXT
    private val textarea = TransitionInputType.TEXTAREA
    private val number = TransitionInputType.NUMBER
    private val checkbox = TransitionInputType.CHECKBOX
    private val json = TransitionInputType.JSON
    private val identityPicker = TransitionInputType.IDENTITY_PICKER
    private val contractPicker = TransitionInputType.CONTRACT_PICKER
    private val docTypePicker = TransitionInputType.DOCUMENT_TYPE_PICKER
    private val docPicker = TransitionInputType.DOCUMENT_PICKER
    private val docWithPrice = TransitionInputType.DOCUMENT_WITH_PRICE
    private val tokenPicker = TransitionInputType.TOKEN_PICKER

    private val voteChoiceOptions = listOf(
        SelectOption("abstain", "Abstain"),
        SelectOption("lock", "Lock"),
        SelectOption("towardsIdentity", "Towards Identity"),
    )

    val all: List<TransitionDefinition> = listOf(
        // ── Identity ──────────────────────────────────────────────────
        TransitionDefinition(
            key = "identityCreate",
            category = TransitionCategory.IDENTITY,
            label = "Create Identity",
            description = "Register a new identity from an asset-lock proof",
            inputs = listOf(
                TransitionInput("identityIndex", number, "Identity index (HD slot)", true, defaultValue = "0"),
                TransitionInput("assetLockProof", textarea, "Asset lock proof", true, help = "The asset lock proof that provides initial credits"),
            ),
        ),
        TransitionDefinition(
            key = "identityTopUp",
            category = TransitionCategory.IDENTITY,
            label = "Top Up Identity",
            description = "Add credits to an identity from Platform addresses",
            inputs = listOf(
                TransitionInput("assetLockProof", textarea, "Asset lock proof", true, help = "The asset lock proof that provides additional credits"),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.TOP_UP_IDENTITY,
        ),
        TransitionDefinition(
            key = "identityUpdate",
            category = TransitionCategory.IDENTITY,
            label = "Update Identity",
            description = "Add or disable identity public keys",
            inputs = listOf(
                TransitionInput("addPublicKeys", textarea, "Add public keys (JSON)", false, help = "JSON array of key rows"),
                TransitionInput("disablePublicKeys", text, "Disable key IDs", false, placeholder = "2,3,5"),
            ),
        ),
        TransitionDefinition(
            key = "identityCreditTransfer",
            category = TransitionCategory.IDENTITY,
            label = "Transfer Credits",
            description = "Transfer credits between identities",
            inputs = listOf(
                TransitionInput("toIdentityId", identityPicker, "Recipient", true),
                TransitionInput("amount", number, "Amount (credits)", true, placeholder = "1000000"),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.TRANSFER_CREDITS,
        ),
        TransitionDefinition(
            key = "identityCreditWithdrawal",
            category = TransitionCategory.IDENTITY,
            label = "Withdraw Credits",
            description = "Withdraw credits to a Core (L1) address",
            inputs = listOf(
                TransitionInput("toAddress", text, "Destination L1 address", true, placeholder = "yXXXX…"),
                TransitionInput("amount", number, "Amount (credits)", true, placeholder = "1000000"),
                TransitionInput("coreFeePerByte", number, "Core fee per byte", false, placeholder = "1"),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.WITHDRAW_CREDITS,
        ),
        // ── Data Contract ─────────────────────────────────────────────
        TransitionDefinition(
            key = "dataContractCreate",
            category = TransitionCategory.DATA_CONTRACT,
            label = "Create Data Contract",
            description = "Deploy a new data contract",
            inputs = listOf(
                TransitionInput("canBeDeleted", checkbox, "Can be deleted", false),
                TransitionInput("readonly", checkbox, "Read-only", false),
                TransitionInput("keepsHistory", checkbox, "Keeps history", false),
                TransitionInput("documentsMutableContractDefault", checkbox, "Documents mutable (default)", false, defaultValue = "true"),
                TransitionInput("documentsCanBeDeletedContractDefault", checkbox, "Documents deletable (default)", false, defaultValue = "true"),
                TransitionInput("documentSchemas", json, "Document schemas (JSON)", false),
                TransitionInput("tokenSchemas", json, "Token schemas (JSON)", false),
                TransitionInput("groups", json, "Groups (JSON)", false),
                TransitionInput("keywords", text, "Keywords", false, placeholder = "Comma-separated keywords"),
                TransitionInput("description", text, "Description", false),
            ),
        ),
        TransitionDefinition(
            key = "dataContractUpdate",
            category = TransitionCategory.DATA_CONTRACT,
            label = "Update Data Contract",
            description = "Add document / token schemas or groups",
            inputs = listOf(
                TransitionInput("dataContractId", text, "Data contract ID", true, placeholder = "Enter data contract ID"),
                TransitionInput("newDocumentSchemas", json, "New document schemas (JSON)", false),
                TransitionInput("newTokenSchemas", json, "New token schemas (JSON)", false),
                TransitionInput("newGroups", json, "New groups (JSON)", false),
            ),
        ),
        // ── Document ──────────────────────────────────────────────────
        TransitionDefinition(
            key = "documentCreate",
            category = TransitionCategory.DOCUMENT,
            label = "Create Document",
            description = "Create a document in a contract",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentFields", json, "Document fields (JSON)", true),
            ),
        ),
        TransitionDefinition(
            key = "documentReplace",
            category = TransitionCategory.DOCUMENT,
            label = "Replace Document",
            description = "Replace an existing document",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", true),
                TransitionInput("documentFields", json, "Document fields (JSON)", true),
            ),
        ),
        TransitionDefinition(
            key = "documentDelete",
            category = TransitionCategory.DOCUMENT,
            label = "Delete Document",
            description = "Delete an existing document",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", true),
            ),
        ),
        TransitionDefinition(
            key = "documentTransfer",
            category = TransitionCategory.DOCUMENT,
            label = "Transfer Document",
            description = "Transfer a document to another identity",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", true),
                TransitionInput("recipientId", identityPicker, "Recipient", true),
            ),
        ),
        TransitionDefinition(
            key = "documentUpdatePrice",
            category = TransitionCategory.DOCUMENT,
            label = "Update Document Price",
            description = "Set or remove a document's marketplace price",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", true),
                TransitionInput("newPrice", number, "New price (credits)", true, help = "0 to remove price"),
            ),
        ),
        TransitionDefinition(
            key = "documentPurchase",
            category = TransitionCategory.DOCUMENT,
            label = "Purchase Document",
            description = "Purchase a listed document",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docWithPrice, "Document ID", true),
            ),
        ),
        // ── Token ─────────────────────────────────────────────────────
        TransitionDefinition(
            key = "tokenMint",
            category = TransitionCategory.TOKEN,
            label = "Mint Tokens",
            description = "Issue new tokens",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("amount", text, "Amount", true),
                TransitionInput("issuedToIdentityId", text, "Issue to (optional)", false),
                TransitionInput("publicNote", text, "Public note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenBurn",
            category = TransitionCategory.TOKEN,
            label = "Burn Tokens",
            description = "Destroy tokens from the owner balance",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("amount", text, "Amount", true),
                TransitionInput("publicNote", text, "Public note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenTransfer",
            category = TransitionCategory.TOKEN,
            label = "Transfer Tokens",
            description = "Transfer tokens to another identity",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("recipientId", text, "Recipient identity ID", true),
                TransitionInput("amount", text, "Amount", true),
                TransitionInput("note", text, "Note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenFreeze",
            category = TransitionCategory.TOKEN,
            label = "Freeze Tokens",
            description = "Freeze an identity's token balance",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("targetIdentityId", text, "Target identity ID", true),
                TransitionInput("note", text, "Note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenUnfreeze",
            category = TransitionCategory.TOKEN,
            label = "Unfreeze Tokens",
            description = "Unfreeze an identity's token balance",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("targetIdentityId", text, "Target identity ID", true),
                TransitionInput("note", text, "Note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenDestroyFrozenFunds",
            category = TransitionCategory.TOKEN,
            label = "Destroy Frozen Funds",
            description = "Destroy a frozen identity's tokens",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput("frozenIdentityId", text, "Frozen identity ID", true),
                TransitionInput("note", text, "Note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenClaim",
            category = TransitionCategory.TOKEN,
            label = "Claim Tokens",
            description = "Claim from a distribution",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput(
                    "distributionType", select, "Distribution type", true,
                    options = listOf(
                        SelectOption("perpetual", "Perpetual"),
                        SelectOption("preprogrammed", "Pre-programmed"),
                    ),
                ),
                TransitionInput("publicNote", text, "Public note", false),
            ),
        ),
        TransitionDefinition(
            key = "tokenSetPrice",
            category = TransitionCategory.TOKEN,
            label = "Set Token Price",
            description = "Set or remove direct-purchase pricing",
            inputs = listOf(
                TransitionInput("token", tokenPicker, "Token", true),
                TransitionInput(
                    "priceType", select, "Price type", true,
                    options = listOf(
                        SelectOption("single", "Single"),
                        SelectOption("tiered", "Tiered"),
                    ),
                ),
                TransitionInput("priceData", text, "Price data", false, help = "Leave empty to remove pricing"),
                TransitionInput("publicNote", text, "Public note", false),
            ),
        ),
        // ── Voting ────────────────────────────────────────────────────
        TransitionDefinition(
            key = "dpnsUsername",
            category = TransitionCategory.VOTING,
            label = "DPNS Username Vote",
            description = "Vote on a contested DPNS username",
            inputs = listOf(
                TransitionInput("contestedUsername", text, "Contested username", true),
                TransitionInput("voteChoice", select, "Vote choice", true, options = voteChoiceOptions),
                TransitionInput("targetIdentity", identityPicker, "Target identity", false),
            ),
        ),
        TransitionDefinition(
            key = "masternodeVote",
            category = TransitionCategory.VOTING,
            label = "Masternode Vote",
            description = "Cast a masternode vote on a contested resource",
            inputs = listOf(
                TransitionInput("contractId", text, "Contract ID", true),
                TransitionInput("documentType", text, "Document type", true),
                TransitionInput("indexName", text, "Index name", true),
                TransitionInput("indexValues", text, "Index values", true, placeholder = "Comma-separated"),
                TransitionInput("voteChoice", select, "Vote choice", true, options = voteChoiceOptions),
                TransitionInput("targetIdentity", identityPicker, "Target identity", false),
            ),
        ),
    )

    /** Definitions in [category], in catalog order. */
    fun forCategory(category: TransitionCategory): List<TransitionDefinition> =
        all.filter { it.category == category }

    /** Look up a definition by its [TransitionDefinition.key]. */
    fun byKey(key: String): TransitionDefinition? = all.firstOrNull { it.key == key }
}
