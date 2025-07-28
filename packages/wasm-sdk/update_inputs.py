#!/usr/bin/env python3
"""
Manually update the inputs for each query/transition based on index.html
"""

import json

# Manually define the inputs for each query based on index.html
query_inputs = {
    "getIdentity": [{"name": "id", "type": "text", "label": "Identity ID", "required": True}],
    "getIdentityKeys": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True},
        {"name": "keyRequestType", "type": "select", "label": "Key Request Type", "required": False, "options": [
            {"value": "all", "label": "All Keys (AllKeys {})"},
            {"value": "specific", "label": "Specific Keys (SpecificKeys with key_ids)"},
            {"value": "search", "label": "Search Keys (SearchKey with purpose_map)"}
        ]}
    ],
    "getIdentitiesContractKeys": [
        {"name": "identitiesIds", "type": "array", "label": "Identity IDs", "required": True},
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "documentTypeName", "type": "text", "label": "Document Type (optional)", "required": False},
        {"name": "keyRequestType", "type": "select", "label": "Key Request Type", "required": False}
    ],
    "getIdentityNonce": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True}
    ],
    "getIdentityContractNonce": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True},
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True}
    ],
    "getIdentityBalance": [
        {"name": "id", "type": "text", "label": "Identity ID", "required": True}
    ],
    "getIdentitiesBalances": [
        {"name": "identityIds", "type": "array", "label": "Identity IDs", "required": True}
    ],
    "getIdentityBalanceAndRevision": [
        {"name": "id", "type": "text", "label": "Identity ID", "required": True}
    ],
    "getIdentityByPublicKeyHash": [
        {"name": "publicKeyHash", "type": "text", "label": "Public Key Hash", "required": True, "placeholder": "b7e904ce25ed97594e72f7af0e66f298031c1754"}
    ],
    "getIdentityByNonUniquePublicKeyHash": [
        {"name": "publicKeyHash", "type": "text", "label": "Public Key Hash", "required": True, "placeholder": "518038dc858461bcee90478fd994bba8057b7531"}
    ],
    "getIdentityTokenBalances": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True},
        {"name": "tokenIds", "type": "array", "label": "Token IDs", "required": True}
    ],
    "getIdentitiesTokenBalances": [
        {"name": "identityIds", "type": "array", "label": "Identity IDs", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token ID", "required": True, "placeholder": "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"}
    ],
    "getIdentityTokenInfos": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True},
        {"name": "tokenIds", "type": "array", "label": "Token IDs (optional)", "required": False, "placeholder": "[\"Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv\"]"}
    ],
    "getIdentitiesTokenInfos": [
        {"name": "identityIds", "type": "array", "label": "Identity IDs", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token ID", "required": True, "placeholder": "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"}
    ],
    "getDataContract": [
        {"name": "id", "type": "text", "label": "Data Contract ID", "required": True, "placeholder": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"}
    ],
    "getDataContractHistory": [
        {"name": "id", "type": "text", "label": "Data Contract ID", "required": True, "placeholder": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"},
        {"name": "limit", "type": "number", "label": "Limit", "required": False},
        {"name": "offset", "type": "number", "label": "Offset", "required": False}
    ],
    "getDataContracts": [
        {"name": "ids", "type": "array", "label": "Data Contract IDs", "required": True, "placeholder": "[\"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec\", \"ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A\"]"}
    ],
    "getDocuments": [
        {"name": "dataContractId", "type": "text", "label": "Data Contract ID", "required": True, "placeholder": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True, "placeholder": "domain"},
        {"name": "whereClause", "type": "text", "label": "Where Clause (JSON)", "required": False, "placeholder": "[[\"normalizedParentDomainName\", \"==\", \"dash\"], [\"normalizedLabel\", \"==\", \"therea1s11mshaddy5\"]]"},
        {"name": "orderBy", "type": "text", "label": "Order By (JSON)", "required": False, "placeholder": "[[\"$createdAt\", \"desc\"]]"},
        {"name": "limit", "type": "number", "label": "Limit", "required": False}
    ],
    "getDocument": [
        {"name": "dataContractId", "type": "text", "label": "Data Contract ID", "required": True, "placeholder": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"},
        {"name": "documentType", "type": "text", "label": "Document Type", "required": True, "placeholder": "domain"},
        {"name": "documentId", "type": "text", "label": "Document ID", "required": True, "placeholder": "7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r"}
    ],
    "getDpnsUsername": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True}
    ],
    "dpnsCheckAvailability": [
        {"name": "label", "type": "text", "label": "Label (Username)", "required": True}
    ],
    "dpnsResolve": [
        {"name": "name", "type": "text", "label": "Name", "required": True}
    ],
    "getContestedResources": [
        {"name": "resultType", "type": "select", "label": "Result Type", "required": True},
        {"name": "documentTypeName", "type": "text", "label": "Document Type", "required": True},
        {"name": "indexName", "type": "text", "label": "Index Name", "required": True},
        {"name": "count", "type": "number", "label": "Count", "required": False}
    ],
    "getContestedResourceVoteState": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "documentTypeName", "type": "text", "label": "Document Type", "required": True},
        {"name": "indexName", "type": "text", "label": "Index Name", "required": True}
    ],
    "getContestedResourceVotersForIdentity": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "documentTypeName", "type": "text", "label": "Document Type", "required": True},
        {"name": "indexName", "type": "text", "label": "Index Name", "required": True},
        {"name": "contestantId", "type": "text", "label": "Contestant Identity ID", "required": True}
    ],
    "getContestedResourceIdentityVotes": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True}
    ],
    "getVotePollsByEndDate": [
        {"name": "startTimeMs", "type": "number", "label": "Start Time (ms)", "required": True},
        {"name": "endTimeMs", "type": "number", "label": "End Time (ms)", "required": True}
    ],
    "getProtocolVersionUpgradeState": [],
    "getProtocolVersionUpgradeVoteStatus": [
        {"name": "startProTxHash", "type": "text", "label": "Start ProTx Hash", "required": True, "placeholder": "143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"},
        {"name": "count", "type": "number", "label": "Count", "required": True}
    ],
    "getEpochsInfo": [
        {"name": "epoch", "type": "number", "label": "Start Epoch", "required": True},
        {"name": "count", "type": "number", "label": "Count", "required": True},
        {"name": "ascending", "type": "checkbox", "label": "Ascending Order", "required": False}
    ],
    "getCurrentEpoch": [],
    "getFinalizedEpochInfos": [
        {"name": "startEpoch", "type": "number", "label": "Start Epoch", "required": True},
        {"name": "count", "type": "number", "label": "Count", "required": True}
    ],
    "getEvonodesProposedEpochBlocksByIds": [
        {"name": "ids", "type": "array", "label": "ProTx Hashes", "required": True, "placeholder": "[\"143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113\"]"}
    ],
    "getEvonodesProposedEpochBlocksByRange": [
        {"name": "startProTxHash", "type": "text", "label": "Start ProTx Hash", "required": True, "placeholder": "143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"},
        {"name": "count", "type": "number", "label": "Count", "required": True}
    ],
    "getTokenStatuses": [
        {"name": "tokenIds", "type": "array", "label": "Token IDs", "required": True}
    ],
    "getTokenDirectPurchasePrices": [
        {"name": "tokenIds", "type": "array", "label": "Token IDs", "required": True}
    ],
    "getTokenContractInfo": [
        {"name": "dataContractId", "type": "text", "label": "Data Contract ID", "required": True, "placeholder": "EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta"}
    ],
    "getTokenPerpetualDistributionLastClaim": [
        {"name": "identityId", "type": "text", "label": "Identity ID", "required": True},
        {"name": "tokenId", "type": "text", "label": "Token ID", "required": True}
    ],
    "getTokenTotalSupply": [
        {"name": "tokenId", "type": "text", "label": "Token ID", "required": True, "placeholder": "Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"}
    ],
    "getGroupInfo": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "groupContractPosition", "type": "number", "label": "Group Contract Position", "required": True}
    ],
    "getGroupInfos": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "startAtGroupContractPosition", "type": "number", "label": "Start at Position", "required": False},
        {"name": "startGroupContractPositionIncluded", "type": "checkbox", "label": "Include Start Position", "required": False},
        {"name": "count", "type": "number", "label": "Count", "required": False}
    ],
    "getGroupActions": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "groupContractPosition", "type": "number", "label": "Group Contract Position", "required": True},
        {"name": "status", "type": "select", "label": "Status", "required": True, "options": [
            {"value": "ACTIVE", "label": "Active"},
            {"value": "CLOSED", "label": "Closed"}
        ]},
        {"name": "startActionId", "type": "text", "label": "Start Action ID", "required": False},
        {"name": "startActionIdIncluded", "type": "checkbox", "label": "Include Start Action", "required": False},
        {"name": "count", "type": "number", "label": "Count", "required": False}
    ],
    "getGroupActionSigners": [
        {"name": "contractId", "type": "text", "label": "Contract ID", "required": True},
        {"name": "groupContractPosition", "type": "number", "label": "Group Contract Position", "required": True},
        {"name": "status", "type": "select", "label": "Status", "required": True, "options": [
            {"value": "ACTIVE", "label": "Active"},
            {"value": "CLOSED", "label": "Closed"}
        ]},
        {"name": "actionId", "type": "text", "label": "Action ID", "required": True}
    ],
    "getStatus": [],
    "getCurrentQuorumsInfo": [],
    "getPrefundedSpecializedBalance": [
        {"name": "identityId", "type": "text", "label": "Specialized Balance ID", "required": True, "placeholder": "AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT"}
    ],
    "getTotalCreditsInPlatform": [],
    "getPathElements": [
        {"name": "path", "type": "array", "label": "Path", "required": True},
        {"name": "keys", "type": "array", "label": "Keys", "required": True}
    ],
    "waitForStateTransitionResult": [
        {"name": "stateTransitionHash", "type": "text", "label": "State Transition Hash", "required": True}
    ]
}

# Load fixed definitions
with open('fixed_definitions.json', 'r') as f:
    definitions = json.load(f)

# Update query inputs
for cat_key, category in definitions['queries'].items():
    for query_key, query in category.get('queries', {}).items():
        if query_key in query_inputs:
            query['inputs'] = query_inputs[query_key]

# Save updated definitions
with open('fixed_definitions.json', 'w') as f:
    json.dump(definitions, f, indent=2)

print("Updated fixed_definitions.json with input parameters")