import Foundation

// MARK: - Transition Definitions

struct TransitionDefinitions {
    static let all: [String: TransitionDefinition] = [
        // Identity Transitions
        "identityCreate": TransitionDefinition(
            key: "identityCreate",
            label: "Identity Create",
            description: "Create a new identity with initial credits",
            inputs: [
                TransitionInput(
                    name: "seedPhrase",
                    type: "textarea",
                    label: "Seed Phrase",
                    required: true,
                    placeholder: "Enter seed phrase (12-24 words) or click Generate",
                    help: "The wallet seed phrase that will be used to derive identity keys"
                ),
                TransitionInput(
                    name: "generateSeedButton",
                    type: "button",
                    label: "Generate New Seed",
                    required: false,
                    action: "generateTestSeed"
                ),
                TransitionInput(
                    name: "identityIndex",
                    type: "number",
                    label: "Identity Index",
                    required: true,
                    help: "The identity index is an internal reference within the wallet. Leave as 0 for first identity.",
                    defaultValue: "0",
                    min: 0,
                    max: 999
                ),
                TransitionInput(
                    name: "assetLockProof",
                    type: "textarea",
                    label: "Asset Lock Proof",
                    required: true,
                    placeholder: "Enter asset lock proof (hex encoded)",
                    help: "The asset lock proof that provides initial credits"
                )
            ]
        ),
        
        "identityTopUp": TransitionDefinition(
            key: "identityTopUp",
            label: "Identity Top Up",
            description: "Add credits to an existing identity",
            inputs: [
                TransitionInput(
                    name: "assetLockProof",
                    type: "textarea",
                    label: "Asset Lock Proof",
                    required: true,
                    placeholder: "Enter asset lock proof (hex encoded)",
                    help: "The asset lock proof that provides additional credits"
                )
            ]
        ),
        
        "identityUpdate": TransitionDefinition(
            key: "identityUpdate",
            label: "Identity Update",
            description: "Update identity keys (add or disable)",
            inputs: [
                TransitionInput(
                    name: "addPublicKeys",
                    type: "textarea",
                    label: "Keys to Add (JSON array)",
                    required: false,
                    placeholder: "[{\"keyType\":\"ECDSA_HASH160\",\"purpose\":\"AUTHENTICATION\",\"data\":\"base64_key_data\"}]"
                ),
                TransitionInput(
                    name: "disablePublicKeys",
                    type: "text",
                    label: "Key IDs to Disable (comma-separated)",
                    required: false,
                    placeholder: "2,3,5"
                )
            ]
        ),
        
        "identityCreditTransfer": TransitionDefinition(
            key: "identityCreditTransfer",
            label: "Identity Credit Transfer",
            description: "Transfer credits between identities",
            inputs: [
                TransitionInput(
                    name: "toIdentityId",
                    type: "identityPicker",
                    label: "Recipient Identity",
                    required: true,
                    placeholder: "Select recipient identity"
                ),
                TransitionInput(
                    name: "amount",
                    type: "number",
                    label: "Amount (credits)",
                    required: true,
                    placeholder: "1000000"
                )
            ]
        ),
        
        "identityCreditWithdrawal": TransitionDefinition(
            key: "identityCreditWithdrawal",
            label: "Identity Credit Withdrawal",
            description: "Withdraw credits to a Dash address",
            inputs: [
                TransitionInput(
                    name: "toAddress",
                    type: "text",
                    label: "Dash Address",
                    required: true,
                    placeholder: "yXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
                ),
                TransitionInput(
                    name: "amount",
                    type: "number",
                    label: "Amount (credits)",
                    required: true,
                    placeholder: "1000000"
                ),
                TransitionInput(
                    name: "coreFeePerByte",
                    type: "number",
                    label: "Core Fee Per Byte (optional)",
                    required: false,
                    placeholder: "1"
                )
            ]
        ),
        
        // Data Contract Transitions
        "dataContractCreate": TransitionDefinition(
            key: "dataContractCreate",
            label: "Data Contract Create",
            description: "Create a new data contract",
            inputs: [
                TransitionInput(
                    name: "canBeDeleted",
                    type: "checkbox",
                    label: "Can Be Deleted",
                    required: false
                ),
                TransitionInput(
                    name: "readonly",
                    type: "checkbox",
                    label: "Read Only",
                    required: false
                ),
                TransitionInput(
                    name: "keepsHistory",
                    type: "checkbox",
                    label: "Keeps History",
                    required: false
                ),
                TransitionInput(
                    name: "documentsKeepHistoryContractDefault",
                    type: "checkbox",
                    label: "Documents Keep History (Default)",
                    required: false
                ),
                TransitionInput(
                    name: "documentsMutableContractDefault",
                    type: "checkbox",
                    label: "Documents Mutable (Default)",
                    required: false,
                    defaultValue: "true"
                ),
                TransitionInput(
                    name: "documentsCanBeDeletedContractDefault",
                    type: "checkbox",
                    label: "Documents Can Be Deleted (Default)",
                    required: false,
                    defaultValue: "true"
                ),
                TransitionInput(
                    name: "documentSchemas",
                    type: "json",
                    label: "Document Schemas JSON",
                    required: true,
                    placeholder: "{\n  \"note\": {\n    \"type\": \"object\",\n    \"properties\": {\n      \"message\": {\n        \"type\": \"string\",\n        \"maxLength\": 100,\n        \"position\": 0\n      }\n    },\n    \"required\": [\"message\"],\n    \"additionalProperties\": false\n  }\n}"
                ),
                TransitionInput(
                    name: "keywords",
                    type: "text",
                    label: "Keywords (comma separated, optional)",
                    required: false
                ),
                TransitionInput(
                    name: "description",
                    type: "text",
                    label: "Description (optional)",
                    required: false
                )
            ]
        ),
        
        "dataContractUpdate": TransitionDefinition(
            key: "dataContractUpdate",
            label: "Data Contract Update",
            description: "Add document types, groups, or tokens to an existing data contract",
            inputs: [
                TransitionInput(
                    name: "dataContractId",
                    type: "text",
                    label: "Data Contract ID",
                    required: true,
                    placeholder: "Enter data contract ID"
                ),
                TransitionInput(
                    name: "newDocumentSchemas",
                    type: "json",
                    label: "New Document Schemas to Add (optional)",
                    required: false,
                    placeholder: "{\n  \"newType\": {\n    \"type\": \"object\",\n    \"properties\": {\n      \"field\": {\n        \"type\": \"string\",\n        \"maxLength\": 100,\n        \"position\": 0\n      }\n    },\n    \"required\": [\"field\"],\n    \"additionalProperties\": false\n  }\n}"
                )
            ]
        ),
        
        // Document Transitions
        "documentCreate": TransitionDefinition(
            key: "documentCreate",
            label: "Document Create",
            description: "Create a new document",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true,
                    placeholder: "Select a contract"
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentFields",
                    type: "json",
                    label: "Document Data",
                    required: true,
                    placeholder: "{\n  \"message\": \"Hello World\"\n}",
                    help: "Enter the document data as JSON. The required fields depend on the selected document type."
                )
            ]
        ),
        
        "documentReplace": TransitionDefinition(
            key: "documentReplace",
            label: "Document Replace",
            description: "Replace an existing document",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentId",
                    type: "documentPicker",
                    label: "Document ID",
                    required: true,
                    placeholder: "Enter or search for document ID"
                ),
                TransitionInput(
                    name: "documentFields",
                    type: "json",
                    label: "Document Data",
                    required: true,
                    placeholder: "{\n  \"message\": \"Updated message\"\n}",
                    help: "Enter the updated document data as JSON"
                )
            ]
        ),
        
        "documentDelete": TransitionDefinition(
            key: "documentDelete",
            label: "Document Delete",
            description: "Delete an existing document",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentId",
                    type: "documentPicker",
                    label: "Document ID",
                    required: true,
                    placeholder: "Enter or search for document ID"
                )
            ]
        ),
        
        "documentTransfer": TransitionDefinition(
            key: "documentTransfer",
            label: "Document Transfer",
            description: "Transfer document ownership",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentId",
                    type: "documentPicker",
                    label: "Document ID",
                    required: true,
                    placeholder: "Enter or search for document ID"
                ),
                TransitionInput(
                    name: "recipientId",
                    type: "identityPicker",
                    label: "Recipient Identity",
                    required: true,
                    placeholder: "" // Will be filled with sender identity to exclude it
                )
            ]
        ),
        
        "documentUpdatePrice": TransitionDefinition(
            key: "documentUpdatePrice",
            label: "Document Update Price",
            description: "Update the price of a document for sale",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentId",
                    type: "documentPicker",
                    label: "Document ID",
                    required: true,
                    placeholder: "Enter document ID to update price"
                ),
                TransitionInput(
                    name: "newPrice",
                    type: "number",
                    label: "New Price (credits)",
                    required: true,
                    help: "The new price for the document in credits (0 to remove price)"
                )
            ]
        ),
        
        "documentPurchase": TransitionDefinition(
            key: "documentPurchase",
            label: "Document Purchase",
            description: "Purchase a document",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "contractPicker",
                    label: "Data Contract",
                    required: true
                ),
                TransitionInput(
                    name: "documentType",
                    type: "documentTypePicker",
                    label: "Document Type",
                    required: true,
                    placeholder: "" // Will be filled with selected contractId
                ),
                TransitionInput(
                    name: "documentId",
                    type: "documentPicker",
                    label: "Document ID",
                    required: true,
                    placeholder: "Enter or search for document ID"
                ),
                TransitionInput(
                    name: "price",
                    type: "number",
                    label: "Price (credits)",
                    required: true,
                    help: "The price to pay for the document in credits"
                )
            ]
        ),
        
        // Token Transitions
        "tokenBurn": TransitionDefinition(
            key: "tokenBurn",
            label: "Token Burn",
            description: "Burn tokens",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "burnableToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "amount",
                    type: "text",
                    label: "Amount to Burn",
                    required: true
                ),
                TransitionInput(
                    name: "publicNote",
                    type: "text",
                    label: "Public Note",
                    required: false
                )
            ]
        ),
        
        "tokenMint": TransitionDefinition(
            key: "tokenMint",
            label: "Token Mint",
            description: "Mint new tokens",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "mintableToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "amount",
                    type: "text",
                    label: "Amount to Mint",
                    required: true
                ),
                TransitionInput(
                    name: "issuedToIdentityId",
                    type: "text",
                    label: "Issue To Identity ID",
                    required: false
                ),
                TransitionInput(
                    name: "publicNote",
                    type: "text",
                    label: "Public Note",
                    required: false
                )
            ]
        ),
        
        "tokenClaim": TransitionDefinition(
            key: "tokenClaim",
            label: "Token Claim",
            description: "Claim tokens from a distribution",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "anyToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "distributionType",
                    type: "select",
                    label: "Distribution Type",
                    required: true,
                    options: [
                        SelectOption(value: "perpetual", label: "Perpetual"),
                        SelectOption(value: "preprogrammed", label: "Pre-programmed")
                    ]
                ),
                TransitionInput(
                    name: "publicNote",
                    type: "text",
                    label: "Public Note",
                    required: false
                )
            ]
        ),
        
        "tokenSetPrice": TransitionDefinition(
            key: "tokenSetPrice",
            label: "Token Set Price",
            description: "Set or update the price for direct token purchases",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "anyToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "priceType",
                    type: "select",
                    label: "Price Type",
                    required: true,
                    options: [
                        SelectOption(value: "single", label: "Single Price"),
                        SelectOption(value: "tiered", label: "Tiered Pricing")
                    ]
                ),
                TransitionInput(
                    name: "priceData",
                    type: "text",
                    label: "Price Data (single price or JSON map)",
                    required: false,
                    placeholder: "Leave empty to remove pricing"
                ),
                TransitionInput(
                    name: "publicNote",
                    type: "text",
                    label: "Public Note",
                    required: false
                )
            ]
        ),
        
        "tokenFreeze": TransitionDefinition(
            key: "tokenFreeze",
            label: "Token Freeze",
            description: "Freeze tokens for a specific identity",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "freezableToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "targetIdentityId",
                    type: "text",
                    label: "Target Identity ID",
                    required: true,
                    placeholder: "Identity ID to freeze tokens for"
                ),
                TransitionInput(
                    name: "note",
                    type: "text",
                    label: "Note",
                    required: false
                )
            ]
        ),
        
        "tokenUnfreeze": TransitionDefinition(
            key: "tokenUnfreeze",
            label: "Token Unfreeze",
            description: "Unfreeze tokens for a specific identity",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "freezableToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "targetIdentityId",
                    type: "text",
                    label: "Target Identity ID",
                    required: true,
                    placeholder: "Identity ID to unfreeze tokens for"
                ),
                TransitionInput(
                    name: "note",
                    type: "text",
                    label: "Note",
                    required: false
                )
            ]
        ),
        
        "tokenDestroyFrozenFunds": TransitionDefinition(
            key: "tokenDestroyFrozenFunds",
            label: "Token Destroy Frozen Funds",
            description: "Destroy frozen funds for a specific identity",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "freezableToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "frozenIdentityId",
                    type: "text",
                    label: "Frozen Identity ID",
                    required: true,
                    placeholder: "Identity ID with frozen tokens to destroy"
                ),
                TransitionInput(
                    name: "note",
                    type: "text",
                    label: "Note",
                    required: false
                )
            ]
        ),
        
        "tokenTransfer": TransitionDefinition(
            key: "tokenTransfer",
            label: "Token Transfer",
            description: "Transfer tokens to another identity",
            inputs: [
                TransitionInput(
                    name: "token",
                    type: "anyToken",
                    label: "Select Token",
                    required: true
                ),
                TransitionInput(
                    name: "recipientId",
                    type: "text",
                    label: "Recipient Identity ID",
                    required: true,
                    placeholder: "Identity ID to transfer tokens to"
                ),
                TransitionInput(
                    name: "amount",
                    type: "text",
                    label: "Amount to Transfer",
                    required: true
                ),
                TransitionInput(
                    name: "note",
                    type: "text",
                    label: "Note",
                    required: false
                )
            ]
        ),
        
        // Voting Transitions
        "dpnsUsername": TransitionDefinition(
            key: "dpnsUsername",
            label: "DPNS Username Vote",
            description: "Cast a vote for a contested DPNS username",
            inputs: [
                TransitionInput(
                    name: "contestedUsername",
                    type: "text",
                    label: "Contested Username",
                    required: true,
                    placeholder: "Enter the contested username (e.g., 'myusername')"
                ),
                TransitionInput(
                    name: "voteChoice",
                    type: "select",
                    label: "Vote Choice",
                    required: true,
                    options: [
                        SelectOption(value: "abstain", label: "Abstain"),
                        SelectOption(value: "lock", label: "Lock (Give to no one)"),
                        SelectOption(value: "towardsIdentity", label: "Vote for Identity")
                    ]
                ),
                TransitionInput(
                    name: "targetIdentity",
                    type: "identityPicker",
                    label: "Target Identity (if voting for identity)",
                    required: false,
                    placeholder: "Select identity to vote for"
                )
            ]
        ),
        
        "masternodeVote": TransitionDefinition(
            key: "masternodeVote",
            label: "Masternode Vote",
            description: "Cast a vote for contested resources as a masternode",
            inputs: [
                TransitionInput(
                    name: "contractId",
                    type: "text",
                    label: "Data Contract ID",
                    required: true,
                    placeholder: "Contract ID containing the contested resource"
                ),
                TransitionInput(
                    name: "fetchContestedResources",
                    type: "button",
                    label: "Get Contested Resources",
                    required: false,
                    action: "fetchContestedResources"
                ),
                TransitionInput(
                    name: "documentType",
                    type: "text",
                    label: "Document Type",
                    required: true
                ),
                TransitionInput(
                    name: "indexName",
                    type: "text",
                    label: "Index Name",
                    required: true
                ),
                TransitionInput(
                    name: "indexValues",
                    type: "text",
                    label: "Index Values (comma-separated)",
                    required: true
                ),
                TransitionInput(
                    name: "voteChoice",
                    type: "select",
                    label: "Vote Choice",
                    required: true,
                    options: [
                        SelectOption(value: "abstain", label: "Abstain"),
                        SelectOption(value: "lock", label: "Lock (Give to no one)"),
                        SelectOption(value: "towardsIdentity", label: "Vote for Identity")
                    ]
                ),
                TransitionInput(
                    name: "targetIdentity",
                    type: "identityPicker",
                    label: "Target Identity (if voting for identity)",
                    required: false,
                    placeholder: "Select identity to vote for"
                )
            ]
        )
    ]
}