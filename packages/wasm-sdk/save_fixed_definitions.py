#!/usr/bin/env python3
"""
Fix the extraction by manually defining the correct structure
"""

import json

# Based on the index.html structure, here's the correct organization
correct_structure = {
    "queries": {
        "identity": {
            "label": "Identity Queries",
            "queries": {
                "getIdentity": {"label": "Get Identity", "description": "Fetch an identity by its identifier"},
                "getIdentityKeys": {"label": "Get Identity Keys", "description": "Retrieve keys associated with an identity"},
                "getIdentitiesContractKeys": {"label": "Get Identities Contract Keys", "description": "Get keys for multiple identities related to a specific contract"},
                "getIdentityNonce": {"label": "Get Identity Nonce", "description": "Get the current nonce for an identity"},
                "getIdentityContractNonce": {"label": "Get Identity Contract Nonce", "description": "Get the nonce for an identity in relation to a specific contract"},
                "getIdentityBalance": {"label": "Get Identity Balance", "description": "Get the credit balance of an identity"},
                "getIdentitiesBalances": {"label": "Get Identities Balances", "description": "Get balances for multiple identities"},
                "getIdentityBalanceAndRevision": {"label": "Get Identity Balance and Revision", "description": "Get both balance and revision number for an identity"},
                "getIdentityByPublicKeyHash": {"label": "Get Identity by Unique Public Key Hash", "description": "Find an identity by its unique public key hash"},
                "getIdentityByNonUniquePublicKeyHash": {"label": "Get Identity by Non-Unique Public Key Hash", "description": "Find identities by non-unique public key hash"},
                "getIdentityTokenBalances": {"label": "Get Identity Token Balances", "description": "Get token balances for an identity"},
                "getIdentitiesTokenBalances": {"label": "Get Identities Token Balances", "description": "Get token balance for multiple identities"},
                "getIdentityTokenInfos": {"label": "Get Identity Token Info", "description": "Get token information for an identity's tokens"},
                "getIdentitiesTokenInfos": {"label": "Get Identities Token Info", "description": "Get token information for multiple identities with a specific token"}
            }
        },
        "dataContract": {
            "label": "Data Contract Queries", 
            "queries": {
                "getDataContract": {"label": "Get Data Contract", "description": "Fetch a data contract by its identifier"},
                "getDataContractHistory": {"label": "Get Data Contract History", "description": "Get the version history of a data contract"},
                "getDataContracts": {"label": "Get Data Contracts", "description": "Fetch multiple data contracts by their identifiers"}
            }
        },
        "document": {
            "label": "Document Queries",
            "queries": {
                "getDocuments": {"label": "Get Documents", "description": "Query documents from a data contract"},
                "getDocument": {"label": "Get Document", "description": "Fetch a specific document by ID"}
            }
        },
        "dpns": {
            "label": "DPNS Queries",
            "queries": {
                "getDpnsUsername": {"label": "Get DPNS Usernames", "description": "Get DPNS usernames for an identity"},
                "dpnsCheckAvailability": {"label": "DPNS Check Availability", "description": "Check if a DPNS username is available"},
                "dpnsResolve": {"label": "DPNS Resolve Name", "description": "Resolve a DPNS name to an identity ID"}
            }
        },
        "voting": {
            "label": "Voting & Contested Resources",
            "queries": {
                "getContestedResources": {"label": "Get Contested Resources", "description": "Get list of contested resources"},
                "getContestedResourceVoteState": {"label": "Get Contested Resource Vote State", "description": "Get the current vote state for a contested resource"},
                "getContestedResourceVotersForIdentity": {"label": "Get Contested Resource Voters for Identity", "description": "Get voters who voted for a specific identity in a contested resource"},
                "getContestedResourceIdentityVotes": {"label": "Get Contested Resource Identity Votes", "description": "Get all votes cast by a specific identity"},
                "getVotePollsByEndDate": {"label": "Get Vote Polls by End Date", "description": "Get vote polls within a time range"}
            }
        },
        "protocol": {
            "label": "Protocol & Version",
            "queries": {
                "getProtocolVersionUpgradeState": {"label": "Get Protocol Version Upgrade State", "description": "Get the current state of protocol version upgrades"},
                "getProtocolVersionUpgradeVoteStatus": {"label": "Get Protocol Version Upgrade Vote Status", "description": "Get voting status for protocol version upgrades"}
            }
        },
        "epoch": {
            "label": "Epoch & Block",
            "queries": {
                "getEpochsInfo": {"label": "Get Epochs Info", "description": "Get information about epochs"},
                "getCurrentEpoch": {"label": "Get Current Epoch", "description": "Get information about the current epoch"},
                "getFinalizedEpochInfos": {"label": "Get Finalized Epoch Info", "description": "Get information about finalized epochs"},
                "getEvonodesProposedEpochBlocksByIds": {"label": "Get Evonodes Proposed Epoch Blocks by IDs", "description": "Get proposed blocks by evonode IDs"},
                "getEvonodesProposedEpochBlocksByRange": {"label": "Get Evonodes Proposed Epoch Blocks by Range", "description": "Get proposed blocks by range"}
            }
        },
        "token": {
            "label": "Token Queries",
            "queries": {
                "getTokenStatuses": {"label": "Get Token Statuses", "description": "Get token statuses"},
                "getTokenDirectPurchasePrices": {"label": "Get Token Direct Purchase Prices", "description": "Get direct purchase prices for tokens"},
                "getTokenContractInfo": {"label": "Get Token Contract Info", "description": "Get information about a token contract"},
                "getTokenPerpetualDistributionLastClaim": {"label": "Get Token Perpetual Distribution Last Claim", "description": "Get last claim information for perpetual distribution"},
                "getTokenTotalSupply": {"label": "Get Token Total Supply", "description": "Get total supply of a token"}
            }
        },
        "group": {
            "label": "Group Queries",
            "queries": {
                "getGroupInfo": {"label": "Get Group Info", "description": "Get information about a group"},
                "getGroupInfos": {"label": "Get Group Infos", "description": "Get information about multiple groups"},
                "getGroupActions": {"label": "Get Group Actions", "description": "Get actions for a group"},
                "getGroupActionSigners": {"label": "Get Group Action Signers", "description": "Get signers for a group action"}
            }
        },
        "system": {
            "label": "System & Utility",
            "queries": {
                "getStatus": {"label": "Get Status", "description": "Get system status"},
                "getCurrentQuorumsInfo": {"label": "Get Current Quorums Info", "description": "Get information about current quorums"},
                "getPrefundedSpecializedBalance": {"label": "Get Prefunded Specialized Balance", "description": "Get prefunded specialized balance"},
                "getTotalCreditsInPlatform": {"label": "Get Total Credits in Platform", "description": "Get total credits in the platform"},
                "getPathElements": {"label": "Get Path Elements", "description": "Get path elements"},
                "waitForStateTransitionResult": {"label": "Wait for State Transition Result", "description": "Wait for a state transition to be processed"}
            }
        }
    },
    "transitions": {
        "identity": {
            "label": "Identity Transitions",
            "transitions": {
                "identityCreate": {"label": "Identity Create", "description": "Create a new identity with initial credits"},
                "identityTopUp": {"label": "Identity Top Up", "description": "Add credits to an existing identity"},
                "identityUpdate": {"label": "Identity Update", "description": "Update identity keys (add or disable)"},
                "identityCreditTransfer": {"label": "Identity Credit Transfer", "description": "Transfer credits between identities"},
                "identityCreditWithdrawal": {"label": "Identity Credit Withdrawal", "description": "Withdraw credits from identity to Dash address"}
            }
        },
        "dataContract": {
            "label": "Data Contract Transitions",
            "transitions": {
                "dataContractCreate": {"label": "Data Contract Create", "description": "Create a new data contract"},
                "dataContractUpdate": {"label": "Data Contract Update", "description": "Add document types, groups, or tokens to an existing data contract"}
            }
        },
        "document": {
            "label": "Document Transitions",
            "transitions": {
                "documentCreate": {"label": "Document Create", "description": "Create a new document"},
                "documentReplace": {"label": "Document Replace", "description": "Replace an existing document"},
                "documentDelete": {"label": "Document Delete", "description": "Delete an existing document"},
                "documentTransfer": {"label": "Document Transfer", "description": "Transfer document ownership"},
                "documentPurchase": {"label": "Document Purchase", "description": "Purchase a document"},
                "documentSetPrice": {"label": "Document Set Price", "description": "Set or update document price"},
                "dpnsRegister": {"label": "DPNS Register Name", "description": "Register a new DPNS username"}
            }
        },
        "token": {
            "label": "Token Transitions",
            "transitions": {
                "tokenBurn": {"label": "Token Burn", "description": "Burn tokens"},
                "tokenMint": {"label": "Token Mint", "description": "Mint new tokens"},
                "tokenTransfer": {"label": "Token Transfer", "description": "Transfer tokens to another identity"},
                "tokenFreeze": {"label": "Token Freeze", "description": "Freeze tokens for an identity"},
                "tokenUnfreeze": {"label": "Token Unfreeze", "description": "Unfreeze tokens for an identity"},
                "tokenDestroyFrozen": {"label": "Token Destroy Frozen Funds", "description": "Destroy frozen tokens"}
            }
        },
        "voting": {
            "label": "Voting Transitions",
            "transitions": {
                "dpnsUsername": {"label": "DPNS Username", "description": "Cast a vote for a contested DPNS username"},
                "masternodeVote": {"label": "Contested Resource", "description": "Cast a vote for contested resources as a masternode"}
            }
        }
    }
}

# Add empty inputs for now - we'll extract these properly later
def add_inputs(obj):
    for cat_key, category in obj.items():
        items_key = 'queries' if 'queries' in category else 'transitions'
        for item_key, item in category.get(items_key, {}).items():
            item['inputs'] = []

add_inputs(correct_structure['queries'])
add_inputs(correct_structure['transitions'])

# Save the corrected structure
with open('fixed_definitions.json', 'w') as f:
    json.dump(correct_structure, f, indent=2)

print("Fixed extraction saved to fixed_definitions.json")
print(f"Categories: {len(correct_structure['queries'])} query categories, {len(correct_structure['transitions'])} transition categories")

# Count items
query_count = sum(len(cat.get('queries', {})) for cat in correct_structure['queries'].values())
trans_count = sum(len(cat.get('transitions', {})) for cat in correct_structure['transitions'].values())
print(f"Total: {query_count} queries, {trans_count} transitions")