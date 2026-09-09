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
     * credits / contract / document-price / token flows have specialized
     * screens rather than the generic builder). Null → executes inline via
     * the generic detail form.
     */
    val dedicatedRoute: DedicatedTransition? = null,
    /**
     * The [org.dashfoundation.example.services.tokens.TokenActionKind.id]
     * a [DedicatedTransition.TOKEN_ACTION] route opens.
     */
    val tokenActionId: String? = null,
)

/** Executable transitions with a dedicated (non-generic-builder) screen. */
enum class DedicatedTransition {
    TRANSFER_CREDITS,
    WITHDRAW_CREDITS,
    TOP_UP_IDENTITY,
    CREATE_IDENTITY,
    REGISTER_CONTRACT,
    UPDATE_CONTRACT,
    CREATE_DOCUMENT,
    DOCUMENT_WITH_PRICE,
    DOCUMENT_ACTIONS,
    TOKEN_ACTION,
    CONTEST_DETAIL,
}

/** The full catalog, keyed by transition key. ← `TransitionDefinitions.all`. */
object StateTransitionDefinitions {

    private val select = TransitionInputType.SELECT
    private val text = TransitionInputType.TEXT
    private val textarea = TransitionInputType.TEXTAREA
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
            inputs = emptyList(),
            executable = true,
            dedicatedRoute = DedicatedTransition.CREATE_IDENTITY,
        ),
        TransitionDefinition(
            key = "identityTopUp",
            category = TransitionCategory.IDENTITY,
            label = "Top Up Identity",
            description = "Add credits to an identity from Platform addresses",
            inputs = listOf(
                TransitionInput("identityId", identityPicker, "Identity to top up", true),
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
                TransitionInput("identityId", identityPicker, "Identity", true),
                TransitionInput(
                    "addPublicKeys", textarea, "Add public keys (JSON)", false,
                    help = "JSON array of key rows — the Add Identity Key screen is " +
                        "the guided path",
                ),
                TransitionInput("disablePublicKeys", text, "Disable key IDs", false, placeholder = "2,3,5"),
            ),
            executable = true,
        ),
        TransitionDefinition(
            key = "identityCreditTransfer",
            category = TransitionCategory.IDENTITY,
            label = "Transfer Credits",
            description = "Transfer credits between identities",
            inputs = listOf(
                TransitionInput("identityId", identityPicker, "Source identity", true),
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
                TransitionInput("identityId", identityPicker, "Source identity", true),
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
                TransitionInput("identityId", identityPicker, "Owner identity", true),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.REGISTER_CONTRACT,
        ),
        TransitionDefinition(
            key = "dataContractUpdate",
            category = TransitionCategory.DATA_CONTRACT,
            label = "Update Data Contract",
            description = "Add document / token schemas or groups",
            inputs = listOf(
                TransitionInput(
                    "dataContractId", contractPicker, "Data contract", false,
                    placeholder = "Enter data contract ID",
                    help = "Optional — the update screen also lets you type or edit the id.",
                ),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.UPDATE_CONTRACT,
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
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.CREATE_DOCUMENT,
        ),
        TransitionDefinition(
            key = "documentReplace",
            category = TransitionCategory.DOCUMENT,
            label = "Replace Document",
            description = "Replace an existing document",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", false),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.DOCUMENT_ACTIONS,
        ),
        TransitionDefinition(
            key = "documentDelete",
            category = TransitionCategory.DOCUMENT,
            label = "Delete Document",
            description = "Delete an existing document",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", false),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.DOCUMENT_ACTIONS,
        ),
        TransitionDefinition(
            key = "documentTransfer",
            category = TransitionCategory.DOCUMENT,
            label = "Transfer Document",
            description = "Transfer a document to another identity",
            inputs = listOf(
                TransitionInput("contractId", contractPicker, "Data contract", true),
                TransitionInput("documentType", docTypePicker, "Document type", true),
                TransitionInput("documentId", docPicker, "Document ID", false),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.DOCUMENT_ACTIONS,
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
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.DOCUMENT_WITH_PRICE,
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
            executable = true,
            dedicatedRoute = DedicatedTransition.DOCUMENT_WITH_PRICE,
        ),
        // ── Token ─────────────────────────────────────────────────────
        // Each token transition routes to its dedicated (fully wired)
        // action form — the [DedicatedTransition.TOKEN_ACTION] pattern; the
        // form collects amounts / targets / notes with the token's rules.
        tokenTransition("tokenMint", "Mint Tokens", "Issue new tokens", "mint"),
        tokenTransition("tokenBurn", "Burn Tokens", "Destroy tokens from the owner balance", "burn"),
        tokenTransition("tokenTransfer", "Transfer Tokens", "Transfer tokens to another identity", "transfer"),
        tokenTransition("tokenFreeze", "Freeze Tokens", "Freeze an identity's token balance", "freeze"),
        tokenTransition("tokenUnfreeze", "Unfreeze Tokens", "Unfreeze an identity's token balance", "unfreeze"),
        tokenTransition("tokenDestroyFrozenFunds", "Destroy Frozen Funds", "Destroy a frozen identity's tokens", "destroyFrozenFunds"),
        tokenTransition("tokenClaim", "Claim Tokens", "Claim from a distribution", "claim"),
        tokenTransition("tokenSetPrice", "Set Token Price", "Set or remove direct-purchase pricing", "setDirectPurchasePrice"),
        // ── Voting ────────────────────────────────────────────────────
        TransitionDefinition(
            key = "dpnsUsername",
            category = TransitionCategory.VOTING,
            label = "DPNS Username Vote",
            description = "Vote on a contested DPNS username",
            inputs = listOf(
                TransitionInput("contestedUsername", text, "Contested username", true),
                TransitionInput("targetIdentity", identityPicker, "Viewing identity", false),
            ),
            executable = true,
            dedicatedRoute = DedicatedTransition.CONTEST_DETAIL,
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
                TransitionInput(
                    "proTxHash", text, "Masternode pro_tx_hash", true,
                    placeholder = "64 hex chars",
                ),
                TransitionInput(
                    "votingPrivateKey", text, "Masternode voting private key", true,
                    placeholder = "64 hex chars",
                    help = "The ECDSA_HASH160 voting key + signer derive from this " +
                        "Rust-side; the bytes are not stored",
                ),
            ),
            executable = true,
        ),
    )

    /** Shared shape of the eight token-transition rows. */
    private fun tokenTransition(
        key: String,
        label: String,
        description: String,
        actionId: String,
    ): TransitionDefinition = TransitionDefinition(
        key = key,
        category = TransitionCategory.TOKEN,
        label = label,
        description = description,
        inputs = listOf(
            TransitionInput("token", tokenPicker, "Token", true),
            TransitionInput("identityId", identityPicker, "Acting identity", true),
        ),
        executable = true,
        dedicatedRoute = DedicatedTransition.TOKEN_ACTION,
        tokenActionId = actionId,
    )

    /** Definitions in [category], in catalog order. */
    fun forCategory(category: TransitionCategory): List<TransitionDefinition> =
        all.filter { it.category == category }

    /** Look up a definition by its [TransitionDefinition.key]. */
    fun byKey(key: String): TransitionDefinition? = all.firstOrNull { it.key == key }
}
