#!/usr/bin/env python3
"""
Extract state transition input definitions from index.html and update fixed_definitions.json
"""

import json
import re

# Manually define the state transition inputs based on index.html stateTransitionDefinitions
state_transition_inputs = {
    "identityCreate": [
        {"name": "publicKeys", "type": "keyArray", "label": "Public Keys", "required": True},
        {"name": "assetLockProof", "type": "assetLockProof", "label": "Asset Lock Proof", "required": True}
    ],
    "identityTopUp": [
        {"name": "assetLockProof", "type": "assetLockProof", "label": "Asset Lock Proof", "required": True}
    ],
    "identityUpdate": [
        {"name": "addPublicKeys", "type": "textarea", "label": "Keys to Add (JSON array)", "required": False, 
         "placeholder": '[{"keyType":"ECDSA_HASH160","purpose":"AUTHENTICATION","data":"base64_key_data"}]'},
        {"name": "disablePublicKeys", "type": "text", "label": "Key IDs to Disable (comma-separated)", "required": False,
         "placeholder": "2,3,5"}
    ],
    "identityCreditTransfer": [
        {"name": "recipientId", "type": "text", "label": "Recipient Identity ID", "required": True},
        {"name": "amount", "type": "number", "label": "Amount (credits)", "required": True}
    ],
    "identityCreditWithdrawal": [
        {"name": "toAddress", "type": "text", "label": "Dash Address", "required": True},
        {"name": "amount", "type": "number", "label": "Amount (credits)", "required": True},
        {"name": "coreFeePerByte", "type": "number", "label": "Core Fee Per Byte (optional)", "required": False}
    ],
    "dataContractCreate": [
        {"name": "canBeDeleted", "type": "checkbox", "label": "Can Be Deleted", "required": False},
        {"name": "readonly", "type": "checkbox", "label": "Read Only", "required": False},
        {"name": "keepsHistory", "type": "checkbox", "label": "Keeps History", "required": False},
        {"name": "documentsKeepHistoryContractDefault", "type": "checkbox", "label": "Documents Keep History (Default)", "required": False},
        {"name": "documentsMutableContractDefault", "type": "checkbox", "label": "Documents Mutable (Default)", "required": False, "defaultValue": True},
        {"name": "documentsCanBeDeletedContractDefault", "type": "checkbox", "label": "Documents Can Be Deleted (Default)", "required": False, "defaultValue": True},
        {"name": "requiresIdentityEncryptionBoundedKey", "type": "text", "label": "Requires Identity Encryption Key (optional)", "required": False},
        {"name": "requiresIdentityDecryptionBoundedKey", "type": "text", "label": "Requires Identity Decryption Key (optional)", "required": False},
        {"name": "documentSchemas", "type": "json", "label": "Document Schemas JSON", "required": True, 
         "placeholder": '{\n  "note": {\n    "type": "object",\n    "properties": {\n      "message": {\n        "type": "string",\n        "maxLength": 100,\n        "position": 0\n      }\n    },\n    "required": ["message"],\n    "additionalProperties": false\n  }\n}'},
        {"name": "groups", "type": "json", "label": "Groups (optional)", "required": False, "placeholder": '{}'},
        {"name": "tokens", "type": "json", "label": "Tokens (optional)", "required": False, "placeholder": '{}'},
        {"name": "keywords", "type": "text", "label": "Keywords (comma separated, optional)", "required": False},
        {"name": "description", "type": "text", "label": "Description (optional)", "required": False}
    ],
    "dataContractUpdate": [
        {"name": "dataContractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "newDocumentSchemas", "type": "json", "label": "New Document Schemas to Add (optional)", "required": False, 
         "placeholder": '{\n  "newType": {\n    "type": "object",\n    "properties": {\n      "field": {\n        "type": "string",\n        "maxLength": 100,\n        "position": 0\n      }\n    },\n    "required": ["field"],\n    "additionalProperties": false\n  }\n}'},
        {"name": "newGroups", "type": "json", "label": "New Groups to Add (optional)", "required": False, "placeholder": '{}'},
        {"name": "newTokens", "type": "json", "label": "New Tokens to Add (optional)", "required": False, "placeholder": '{}'}
    ],
    "documentCreate": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "fetchSchema", "type": "button", "label": "Fetch Schema", "action": "fetchDocumentSchema"},
        {"name": "documentFields", "type": "dynamic", "label": "Document Fields"}
    ],
    "documentReplace": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True},
        {"name": "loadDocument", "type": "button", "label": "Load Document", "action": "loadExistingDocument"},
        {"name": "documentFields", "type": "dynamic", "label": "Document Fields"}
    ],
    "documentDelete": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True}
    ],
    "documentTransfer": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True},
        {"name": "recipientId", "type": "text", "label": "Recipient Identity ID", "required": True}
    ],
    "documentPurchase": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True},
        {"name": "price", "type": "number", "label": "Price (credits)", "required": True}
    ],
    "documentSetPrice": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True},
        {"name": "price", "type": "number", "label": "Price (credits, 0 to remove)", "required": True}
    ],
    "dpnsRegister": [
        {"name": "label", "type": "text", "label": "Username", "required": True, 
         "placeholder": "Enter username (e.g., alice)", "validateOnType": True}
    ],
    "tokenBurn": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenPosition", "type": "number", "label": "Token Contract Position", "required": True},
        {"name": "amount", "type": "text", "label": "Amount to Burn", "required": True},
        {"name": "keyId", "type": "number", "label": "Key ID (for signing)", "required": True},
        {"name": "publicNote", "type": "text", "label": "Public Note", "required": False}
    ],
    "tokenMint": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenPosition", "type": "number", "label": "Token Contract Position", "required": True},
        {"name": "amount", "type": "text", "label": "Amount to Mint", "required": True},
        {"name": "keyId", "type": "number", "label": "Key ID (for signing)", "required": True},
        {"name": "issuedToIdentityId", "type": "text", "label": "Issue To Identity ID", "required": False},
        {"name": "publicNote", "type": "text", "label": "Public Note", "required": False}
    ],
    "tokenTransfer": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token Contract Position", "required": True},
        {"name": "amount", "type": "number", "label": "Amount to Transfer", "required": True},
        {"name": "recipientId", "type": "text", "label": "Recipient Identity ID", "required": True}
    ],
    "tokenFreeze": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token Contract Position", "required": True},
        {"name": "identityId", "type": "text", "label": "Identity ID to Freeze", "required": True}
    ],
    "tokenUnfreeze": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token Contract Position", "required": True},
        {"name": "identityId", "type": "text", "label": "Identity ID to Unfreeze", "required": True}
    ],
    "tokenDestroyFrozen": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token Contract Position", "required": True},
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True}
    ],
    "dpnsUsername": [
        {"name": "contestedUsername", "type": "text", "label": "Contested Username", "required": True,
         "placeholder": "Enter the contested username (e.g., 'myusername')"},
        {"name": "voteChoice", "type": "select", "label": "Vote Choice", "required": True,
         "options": [
             {"value": "abstain", "label": "Abstain"},
             {"value": "lock", "label": "Lock (Give to no one)"},
             {"value": "towardsIdentity", "label": "Vote for Identity"}
         ]},
        {"name": "targetIdentity", "type": "text", "label": "Target Identity ID (if voting for identity)", "required": False,
         "placeholder": "Identity ID to vote for",
         "dependsOn": {"field": "voteChoice", "value": "towardsIdentity"}}
    ],
    "masternodeVote": [
        {"name": "contractId", "type": "text", "label": "Data Contract ID", "required": True,
         "placeholder": "Contract ID containing the contested resource"},
        {"name": "fetchContestedResources", "type": "button", "label": "Get Contested Resources", "action": "fetchContestedResources"},
        {"name": "contestedResourceDropdown", "type": "dynamic", "label": "Contested Resources"},
        {"name": "voteChoice", "type": "select", "label": "Vote Choice", "required": True,
         "options": [
             {"value": "abstain", "label": "Abstain"},
             {"value": "lock", "label": "Lock (Give to no one)"},
             {"value": "towardsIdentity", "label": "Vote for Identity"}
         ]},
        {"name": "targetIdentity", "type": "text", "label": "Target Identity ID (if voting for identity)", "required": False,
         "placeholder": "Identity ID to vote for",
         "dependsOn": {"field": "voteChoice", "value": "towardsIdentity"}}
    ]
}

# Load fixed definitions
with open('fixed_definitions.json', 'r') as f:
    definitions = json.load(f)

# Update state transition inputs
for cat_key, category in definitions['transitions'].items():
    for trans_key, transition in category.get('transitions', {}).items():
        if trans_key in state_transition_inputs:
            transition['inputs'] = state_transition_inputs[trans_key]
            print(f"Updated inputs for {trans_key}: {len(state_transition_inputs[trans_key])} parameters")
        else:
            print(f"Warning: No inputs defined for {trans_key}")

# Save updated definitions
with open('fixed_definitions.json', 'w') as f:
    json.dump(definitions, f, indent=2)

print("Updated fixed_definitions.json with state transition input parameters")