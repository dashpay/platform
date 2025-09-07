# Dash Platform WASM JS SDK - AI Reference

## Overview
The Dash Platform WASM JS SDK provides WebAssembly bindings for interacting with Dash Platform from JavaScript/TypeScript. This reference is optimized for AI understanding and quick implementation.

## Quick Setup
```javascript
// Import and initialize
import init, { WasmSdk } from './pkg/wasm_sdk.js';

await init();
const transport = { 
    url: "https://52.12.176.90:1443/", // testnet
    network: "testnet"
};
const proofs = true; // Enable proof verification
const sdk = await WasmSdk.new(transport, proofs);
```

## Authentication
Most state transitions require authentication:
```javascript
const identityHex = "hex_encoded_identity";
const privateKeyHex = "hex_encoded_private_key";
```

## Query Operations

### Pattern
All queries follow this pattern:
```javascript
const result = await sdk.{query_name}(param1, param2, ...);
```

### Available Queries

#### Identity Queries

**Get Identity** - `getIdentity`
*Fetch an identity by its identifier*

Parameters:
- `id` (text, required) - Identity ID

Example:
```javascript
const identity = await sdk.getIdentity("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
```

**Get Identity Keys** - `getIdentityKeys`
*Retrieve keys associated with an identity*

Parameters:
- `identityId` (text, required) - Identity ID
- `keyRequestType` (select, optional) - Key Request Type
  - Options: `all` (All Keys (AllKeys {}) - Get all keys for the identity), `specific` (Specific Keys (SpecificKeys with key_ids) - Get specific keys by ID [🚧 Work in Progress]), `search` (Search Keys (SearchKey with purpose_map) - Search by purpose and security level [🚧 Work in Progress])
- `specificKeyIds` (array, optional) - Specific Key IDs (required for 'specific' type)
  - Example: `0,1,2`
- `searchPurposeMap` (text, optional) - Search Purpose Map JSON (required for 'search' type)
  - Example: `{"0": {"0": "current"}, "1": {"0": "all"}}`
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset

Example:
```javascript
const result = await sdk.getIdentityKeys("identityId");
```

**Get Identities Contract Keys** - `getIdentitiesContractKeys`
*Get keys for multiple identities related to a specific contract*

Parameters:
- `identitiesIds` (array, required) - Identity IDs
- `contractId` (text, required) - Contract ID
- `purposes` (multiselect, optional) - Key Purposes
  - Options: `0` (Authentication), `1` (Encryption), `2` (Decryption), `3` (Transfer), `5` (Voting)

Example:
```javascript
const result = await sdk.getIdentitiesContractKeys([], "contractId");
```

**Get Identity Nonce** - `getIdentityNonce`
*Get the current nonce for an identity*

Parameters:
- `identityId` (text, required) - Identity ID

Example:
```javascript
const result = await sdk.getIdentityNonce("identityId");
```

**Get Identity Contract Nonce** - `getIdentityContractNonce`
*Get the nonce for an identity in relation to a specific contract*

Parameters:
- `identityId` (text, required) - Identity ID
- `contractId` (text, required) - Contract ID

Example:
```javascript
const result = await sdk.getIdentityContractNonce("identityId", "contractId");
```

**Get Identity Balance** - `getIdentityBalance`
*Get the credit balance of an identity*

Parameters:
- `id` (text, required) - Identity ID

Example:
```javascript
const balance = await sdk.getIdentityBalance(identityId);
```

**Get Identities Balances** - `getIdentitiesBalances`
*Get balances for multiple identities*

Parameters:
- `ids` (array, required) - Identity IDs

Example:
```javascript
const result = await sdk.getIdentitiesBalances([]);
```

**Get Identity Balance and Revision** - `getIdentityBalanceAndRevision`
*Get both balance and revision number for an identity*

Parameters:
- `id` (text, required) - Identity ID

Example:
```javascript
const result = await sdk.getIdentityBalanceAndRevision("id");
```

**Get Identity by Unique Public Key Hash** - `getIdentityByPublicKeyHash`
*Find an identity by its unique public key hash*

Parameters:
- `publicKeyHash` (text, required) - Public Key Hash
  - Example: `b7e904ce25ed97594e72f7af0e66f298031c1754`

Example:
```javascript
const result = await sdk.getIdentityByPublicKeyHash("publicKeyHash");
```

**Get Identity by Non-Unique Public Key Hash** - `getIdentityByNonUniquePublicKeyHash`
*Find identities by non-unique public key hash*

Parameters:
- `publicKeyHash` (text, required) - Public Key Hash
  - Example: `518038dc858461bcee90478fd994bba8057b7531`
- `startAfter` (text, optional) - Start After

Example:
```javascript
const result = await sdk.getIdentityByNonUniquePublicKeyHash("publicKeyHash");
```

**Get Identity Token Balances** - `getIdentityTokenBalances`
*Get token balances for an identity*

Parameters:
- `identityId` (text, required) - Identity ID
- `tokenIds` (array, required) - Token IDs

Example:
```javascript
const result = await sdk.getIdentityTokenBalances("identityId", []);
```

**Get Identities Token Balances** - `getIdentitiesTokenBalances`
*Get token balance for multiple identities*

Parameters:
- `identityIds` (array, required) - Identity IDs
- `tokenId` (text, required) - Token ID
  - Example: `Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv`

Example:
```javascript
const result = await sdk.getIdentitiesTokenBalances([], "tokenId");
```

**Get Identity Token Info** - `getIdentityTokenInfos`
*Get token information for an identity's tokens*

Parameters:
- `identityId` (text, required) - Identity ID
- `tokenIds` (array, optional) - Token IDs (optional)
  - Example: `["Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv"]`
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset

Example:
```javascript
const result = await sdk.getIdentityTokenInfos("identityId");
```

**Get Identities Token Info** - `getIdentitiesTokenInfos`
*Get token information for multiple identities with a specific token*

Parameters:
- `identityIds` (array, required) - Identity IDs
- `tokenId` (text, required) - Token ID
  - Example: `Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv`

Example:
```javascript
const result = await sdk.getIdentitiesTokenInfos([], "tokenId");
```

#### Data Contract Queries

**Get Data Contract** - `getDataContract`
*Fetch a data contract by its identifier*

Parameters:
- `id` (text, required) - Data Contract ID
  - Example: `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`

Example:
```javascript
const result = await sdk.getDataContract("id");
```

**Get Data Contract History** - `getDataContractHistory`
*Get the version history of a data contract*

Parameters:
- `id` (text, required) - Data Contract ID
  - Example: `HLY575cNazmc5824FxqaEMEBuzFeE4a98GDRNKbyJqCM`
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset
- `startAtMs` (number, optional) - Start At Timestamp (ms)

Example:
```javascript
const result = await sdk.getDataContractHistory("id");
```

**Get Data Contracts** - `getDataContracts`
*Fetch multiple data contracts by their identifiers*

Parameters:
- `ids` (array, required) - Data Contract IDs
  - Example: `["GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec", "ALybvzfcCwMs7sinDwmtumw17NneuW7RgFtFHgjKmF3A"]`

Example:
```javascript
const result = await sdk.getDataContracts([]);
```

#### Document Queries

**Get Documents** - `getDocuments`
*Query documents from a data contract*

Parameters:
- `dataContractId` (text, required) - Data Contract ID
  - Example: `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`
- `documentType` (text, required) - Document Type
  - Example: `domain`
- `where` (json, optional) - Where Clause (JSON)
  - Example: `[["normalizedParentDomainName", "==", "dash"], ["normalizedLabel", "==", "therea1s11mshaddy5"]]`
- `orderBy` (json, optional) - Order By (JSON)
  - Example: `[["$createdAt", "desc"]]`
- `limit` (number, optional) - Limit

Example:
```javascript
const docs = await sdk.getDocuments(
    contractId,
    "note",
    JSON.stringify([["$ownerId", "==", identityId]]),
    JSON.stringify([["$createdAt", "desc"]]),
    10
);
```

**Get Document** - `getDocument`
*Fetch a specific document by ID*

Parameters:
- `dataContractId` (text, required) - Data Contract ID
  - Example: `GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec`
- `documentType` (text, required) - Document Type
  - Example: `domain`
- `documentId` (text, required) - Document ID
  - Example: `7NYmEKQsYtniQRUmxwdPGeVcirMoPh5ZPyAKz8BWFy3r`

Example:
```javascript
const result = await sdk.getDocument("dataContractId", "documentType", "documentId");
```

#### DPNS Queries

**Get DPNS Usernames** - `getDpnsUsername`
*Get DPNS usernames for an identity*

Parameters:
- `identityId` (text, required) - Identity ID

Example:
```javascript
const result = await sdk.getDpnsUsername("identityId");
```

**DPNS Check Availability** - `dpnsCheckAvailability`
*Check if a DPNS username is available*

Parameters:
- `label` (text, required) - Label (Username)

Example:
```javascript
const result = await sdk.dpnsCheckAvailability("label");
```

**DPNS Resolve Name** - `dpnsResolve`
*Resolve a DPNS name to an identity ID*

Parameters:
- `name` (text, required) - Name

Example:
```javascript
const result = await sdk.dpnsResolve("name");
```

**DPNS Search Name** - `dpnsSearch`
*Search for DPNS names that start with a given prefix*

Parameters:
- `prefix` (text, required) - Name Prefix
  - Example: `Enter prefix (e.g., ali)`
- `limit` (number, optional) - Limit
  - Example: `Default: 10`

Example:
```javascript
const result = await sdk.dpnsSearch("prefix");
```

#### Voting & Contested Resources

**Get Contested Resources** - `getContestedResources`
*Get list of contested resources*

Parameters:
- `documentTypeName` (text, required) - Document Type
- `dataContractId` (text, required) - Data Contract ID
- `indexName` (text, required) - Index Name
- `startAtValue` (text, optional) - Start At Value
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getContestedResources("documentTypeName", "dataContractId", "indexName");
```

**Get Contested Resource Vote State** - `getContestedResourceVoteState`
*Get the current vote state for a contested resource*

Parameters:
- `dataContractId` (text, required) - Data Contract ID
- `documentTypeName` (text, required) - Document Type
- `indexName` (text, required) - Index Name
- `indexValues` (array, required) - Index Values
  - Example: `["dash", "alice"]`
- `resultType` (text, required) - Result Type
- `allowIncludeLockedAndAbstainingVoteTally` (checkbox, optional) - Allow Include Locked and Abstaining Vote Tally
- `startAtIdentifierInfo` (text, optional) - Start At Identifier Info
- `count` (number, optional) - Count
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getContestedResourceVoteState("dataContractId", "documentTypeName", "indexName", [], "resultType");
```

**Get Contested Resource Voters for Identity** - `getContestedResourceVotersForIdentity`
*Get voters who voted for a specific identity in a contested resource*

Parameters:
- `dataContractId` (text, required) - Contract ID
- `documentTypeName` (text, required) - Document Type
- `indexName` (text, required) - Index Name
- `indexValues` (array, required) - Index Values
  - Example: `["dash", "alice"]`
- `contestantId` (text, required) - Contestant Identity ID
- `startAtIdentifierInfo` (text, optional) - Start At Identifier Info
- `count` (number, optional) - Count
  - Example: `Default: 100`
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getContestedResourceVotersForIdentity("dataContractId", "documentTypeName", "indexName", [], "contestantId");
```

**Get Contested Resource Identity Votes** - `getContestedResourceIdentityVotes`
*Get all votes cast by a specific identity*

Parameters:
- `identityId` (text, required) - Identity ID
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getContestedResourceIdentityVotes("identityId");
```

**Get Vote Polls by End Date** - `getVotePollsByEndDate`
*Get vote polls within a time range*

Parameters:
- `startTimeMs` (number, optional) - Start Time (ms)
  - Example: `Timestamp in milliseconds as string (e.g., 1650000000000)`
- `endTimeMs` (number, optional) - End Time (ms)
  - Example: `Timestamp in milliseconds as string (e.g., 1650086400000)`
- `limit` (number, optional) - Limit
- `offset` (number, optional) - Offset
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getVotePollsByEndDate();
```

#### Protocol & Version

**Get Protocol Version Upgrade State** - `getProtocolVersionUpgradeState`
*Get the current state of protocol version upgrades*

No parameters required.

Example:
```javascript
const result = await sdk.getProtocolVersionUpgradeState();
```

**Get Protocol Version Upgrade Vote Status** - `getProtocolVersionUpgradeVoteStatus`
*Get voting status for protocol version upgrades*

Parameters:
- `startProTxHash` (text, required) - Start ProTx Hash
  - Example: `143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113`
- `count` (number, required) - Count

Example:
```javascript
const result = await sdk.getProtocolVersionUpgradeVoteStatus("startProTxHash", 100);
```

#### Epoch & Block

**Get Epochs Info** - `getEpochsInfo`
*Get information about epochs*

Parameters:
- `startEpoch` (number, required) - Start Epoch
- `count` (number, required) - Count
- `ascending` (checkbox, optional) - Ascending Order

Example:
```javascript
const result = await sdk.getEpochsInfo(100, 100);
```

**Get Current Epoch** - `getCurrentEpoch`
*Get information about the current epoch*

No parameters required.

Example:
```javascript
const result = await sdk.getCurrentEpoch();
```

**Get Finalized Epoch Info** - `getFinalizedEpochInfos`
*Get information about finalized epochs*

Parameters:
- `startEpoch` (number, required) - Start Epoch
- `count` (number, required) - Count
- `ascending` (checkbox, optional) - Ascending Order

Example:
```javascript
const result = await sdk.getFinalizedEpochInfos(100, 100);
```

**Get Evonodes Proposed Epoch Blocks by IDs** - `getEvonodesProposedEpochBlocksByIds`
*Get proposed blocks by evonode IDs*

Parameters:
- `epoch` (number, required) - Epoch
- `ids` (array, required) - ProTx Hashes
  - Example: `["143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113"]`

Example:
```javascript
const result = await sdk.getEvonodesProposedEpochBlocksByIds(100, []);
```

**Get Evonodes Proposed Epoch Blocks by Range** - `getEvonodesProposedEpochBlocksByRange`
*Get proposed blocks by range*

Parameters:
- `epoch` (number, required) - Epoch
- `limit` (number, optional) - Limit
- `startAfter` (text, optional) - Start After (Evonode ID)
  - Example: `143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113`
- `orderAscending` (checkbox, optional) - Order Ascending

Example:
```javascript
const result = await sdk.getEvonodesProposedEpochBlocksByRange(100);
```

#### Token Queries

**Get Token Statuses** - `getTokenStatuses`
*Get token statuses*

Parameters:
- `tokenIds` (array, required) - Token IDs

Example:
```javascript
const result = await sdk.getTokenStatuses([]);
```

**Get Token Direct Purchase Prices** - `getTokenDirectPurchasePrices`
*Get direct purchase prices for tokens*

Parameters:
- `tokenIds` (array, required) - Token IDs

Example:
```javascript
const result = await sdk.getTokenDirectPurchasePrices([]);
```

**Get Token Contract Info** - `getTokenContractInfo`
*Get information about a token contract*

Parameters:
- `dataContractId` (text, required) - Data Contract ID
  - Example: `EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta`

Example:
```javascript
const result = await sdk.getTokenContractInfo("dataContractId");
```

**Get Token Perpetual Distribution Last Claim** - `getTokenPerpetualDistributionLastClaim`
*Get last claim information for perpetual distribution*

Parameters:
- `identityId` (text, required) - Identity ID
- `tokenId` (text, required) - Token ID

Example:
```javascript
const result = await sdk.getTokenPerpetualDistributionLastClaim("identityId", "tokenId");
```

**Get Token Total Supply** - `getTokenTotalSupply`
*Get total supply of a token*

Parameters:
- `tokenId` (text, required) - Token ID
  - Example: `Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv`

Example:
```javascript
const result = await sdk.getTokenTotalSupply("tokenId");
```

#### Group Queries

**Get Group Info** - `getGroupInfo`
*Get information about a group*

Parameters:
- `contractId` (text, required) - Contract ID
- `groupContractPosition` (number, required) - Group Contract Position

Example:
```javascript
const result = await sdk.getGroupInfo("contractId", 100);
```

**Get Group Infos** - `getGroupInfos`
*Get information about multiple groups*

Parameters:
- `contractId` (text, required) - Contract ID
- `startAtGroupContractPosition` (number, optional) - Start at Position
- `startGroupContractPositionIncluded` (checkbox, optional) - Include Start Position
- `count` (number, optional) - Count

Example:
```javascript
const result = await sdk.getGroupInfos("contractId");
```

**Get Group Actions** - `getGroupActions`
*Get actions for a group*

Parameters:
- `contractId` (text, required) - Contract ID
- `groupContractPosition` (number, required) - Group Contract Position
- `status` (select, required) - Status
  - Options: `ACTIVE` (Active), `CLOSED` (Closed)
- `startActionId` (text, optional) - Start Action ID
- `startActionIdIncluded` (checkbox, optional) - Include Start Action
- `count` (number, optional) - Count

Example:
```javascript
const result = await sdk.getGroupActions("contractId", 100, "status");
```

**Get Group Action Signers** - `getGroupActionSigners`
*Get signers for a group action*

Parameters:
- `contractId` (text, required) - Contract ID
- `groupContractPosition` (number, required) - Group Contract Position
- `status` (select, required) - Status
  - Options: `ACTIVE` (Active), `CLOSED` (Closed)
- `actionId` (text, required) - Action ID

Example:
```javascript
const result = await sdk.getGroupActionSigners("contractId", 100, "status", "actionId");
```

#### System & Utility

**Get Status** - `getStatus`
*Get system status*

No parameters required.

Example:
```javascript
const result = await sdk.getStatus();
```

**Get Current Quorums Info** - `getCurrentQuorumsInfo`
*Get information about current quorums*

No parameters required.

Example:
```javascript
const result = await sdk.getCurrentQuorumsInfo();
```

**Get Prefunded Specialized Balance** - `getPrefundedSpecializedBalance`
*Get prefunded specialized balance*

Parameters:
- `identityId` (text, required) - Specialized Balance ID
  - Example: `AzaU7zqCT7X1kxh8yWxkT9PxAgNqWDu4Gz13emwcRyAT`

Example:
```javascript
const result = await sdk.getPrefundedSpecializedBalance("identityId");
```

**Get Total Credits in Platform** - `getTotalCreditsInPlatform`
*Get total credits in the platform*

No parameters required.

Example:
```javascript
const result = await sdk.getTotalCreditsInPlatform();
```

**Get Path Elements** - `getPathElements`
*Access any data in the Dash Platform state tree. This low-level query allows direct access to GroveDB storage by specifying a path through the tree structure and keys to retrieve at that path. Common paths include: Identities (32), Tokens (96), DataContractDocuments (64), Balances (16), Votes (80), and more.*

Parameters:
- `path` (array, required) - Path
- `keys` (array, required) - Keys

Example:
```javascript
// Access any data in the Dash Platform state tree
// Common root paths:
// - DataContractDocuments: 64
// - Identities: 32
// - UniquePublicKeyHashesToIdentities: 24
// - NonUniquePublicKeyKeyHashesToIdentities: 8
// - Tokens: 16
// - Pools: 48
// - PreFundedSpecializedBalances: 40
// - SpentAssetLockTransactions: 72
// - WithdrawalTransactions: 80
// - GroupActions: 88
// - Balances: 96
// - Misc: 104
// - Votes: 112
// - Versions: 120

// Example: Get identity balance
const result = await sdk.getPathElements(['96'], ['identityId']);

// Example: Get identity info
const identityKeys = await sdk.getPathElements(['32'], ['identityId']);

// Example: Get contract documents
const documents = await sdk.getPathElements(['64'], ['contractId', '1', 'documentType']);
```

**Wait for State Transition Result** - `waitForStateTransitionResult`
*Internal query to wait for and retrieve the result of a previously submitted state transition*

Parameters:
- `stateTransitionHash` (text, required) - State Transition Hash

Example:
```javascript
const result = await sdk.waitForStateTransitionResult("stateTransitionHash");
```

## State Transition Operations

### Pattern
All state transitions require authentication and follow this pattern:
```javascript
const result = await sdk.{transition_name}(identityHex, ...params, privateKeyHex);
```

### Available State Transitions

#### Identity Transitions

**Identity Create** - `identityCreate`
*Create a new identity with initial credits*

Parameters:
- `assetLockProof` (string, required) - Asset Lock Proof
  - Hex-encoded JSON asset lock proof
- `assetLockProofPrivateKey` (string, required) - Asset Lock Proof Private Key
  - WIF format private key
- `publicKeys` (string, required) - Public Keys
  - JSON array of public keys. Key requirements: ECDSA_SECP256K1 requires privateKeyHex or privateKeyWif for signing, BLS12_381 requires privateKeyHex for signing, ECDSA_HASH160 requires either the data field (base64-encoded 20-byte public key hash) or privateKeyHex (produces empty signatures).

Example:
```javascript
// Asset lock proof is a hex-encoded JSON object
const assetLockProof = "a9147d3b... (hex-encoded)";
const assetLockProofPrivateKey = "XFfpaSbZq52HPy3WWwe1dXsZMiU1bQn8vQd34HNXkSZThevBWRn1"; // WIF format

// Public keys array with proper key types and private keys for signing/hashing
const publicKeys = JSON.stringify([
  {
    id: 0,
    keyType: "ECDSA_SECP256K1",
    purpose: "AUTHENTICATION",
    securityLevel: "MASTER",
    data: "A5GzYHPIolbHkFrp5l+s9IvF2lWMuuuSu3oWZB8vWHNJ", // Base64-encoded public key
    readOnly: false,
    privateKeyWif: "XBrZJKcW4ajWVNAU6yP87WQog6CjFnpbqyAKgNTZRqmhYvPgMNV2"
  },
  {
    id: 1,
    keyType: "ECDSA_HASH160",
    purpose: "AUTHENTICATION",
    securityLevel: "HIGH",
    data: "ripemd160hash20bytes1234", // Base64-encoded 20-byte RIPEMD160 hash
    readOnly: false,
    // ECDSA_HASH160 keys produce empty signatures (not required/used for signing)
  }
]);

const result = await sdk.identityCreate(assetLockProof, assetLockProofPrivateKey, publicKeys);
```

**Identity Top Up** - `identityTopUp`
*Add credits to an existing identity*

Parameters:
- `identityId` (string, required) - Identity ID
  - Base58 format identity ID
- `assetLockProof` (string, required) - Asset Lock Proof
  - Hex-encoded JSON asset lock proof
- `assetLockProofPrivateKey` (string, required) - Asset Lock Proof Private Key
  - WIF format private key

Example:
```javascript
const identityId = "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk"; // base58
const assetLockProof = "a9147d3b... (hex-encoded)";
const assetLockProofPrivateKey = "XFfpaSbZq52HPy3WWve1dXsZMiU1bQn8vQd34HNXkSZThevBWRn1"; // WIF format

const result = await sdk.identityTopUp(identityId, assetLockProof, assetLockProofPrivateKey);
```

**Identity Update** - `identityUpdate`
*Update identity keys (add or disable)*

Parameters (in addition to identity/key):
- `addPublicKeys` (textarea, optional) - Keys to Add (JSON array)
  - Example: `[{"keyType":"ECDSA_HASH160","purpose":"AUTHENTICATION","data":"base64_key_data"}]`
- `disablePublicKeys` (text, optional) - Key IDs to Disable (comma-separated)
  - Example: `2,3,5`

Example:
```javascript
const result = await sdk.identityUpdate(identityHex, /* params */, privateKeyHex);
```

**Identity Credit Transfer** - `identityCreditTransfer`
*Transfer credits between identities*

Parameters (in addition to identity/key):
- `recipientId` (text, required) - Recipient Identity ID
- `amount` (number, required) - Amount (credits)

Example:
```javascript
const result = await sdk.identityCreditTransfer(identityHex, /* params */, privateKeyHex);
```

**Identity Credit Withdrawal** - `identityCreditWithdrawal`
*Withdraw credits from identity to Dash address*

Parameters (in addition to identity/key):
- `toAddress` (text, required) - Dash Address
- `amount` (number, required) - Amount (credits)
- `coreFeePerByte` (number, optional) - Core Fee Per Byte (optional)

Example:
```javascript
const result = await sdk.identityCreditWithdrawal(identityHex, /* params */, privateKeyHex);
```

#### Data Contract Transitions

**Data Contract Create** - `dataContractCreate`
*Create a new data contract*

Parameters (in addition to identity/key):
- `canBeDeleted` (checkbox, optional) - Can Be Deleted
- `readonly` (checkbox, optional) - Read Only
- `keepsHistory` (checkbox, optional) - Keeps History
- `documentsKeepHistoryContractDefault` (checkbox, optional) - Documents Keep History (Default)
- `documentsMutableContractDefault` (checkbox, optional) - Documents Mutable (Default)
- `documentsCanBeDeletedContractDefault` (checkbox, optional) - Documents Can Be Deleted (Default)
- `requiresIdentityEncryptionBoundedKey` (text, optional) - Requires Identity Encryption Key (optional)
- `requiresIdentityDecryptionBoundedKey` (text, optional) - Requires Identity Decryption Key (optional)
- `documentSchemas` (json, required) - Document Schemas JSON
  - Example: `{
  "note": {
    "type": "object",
    "properties": {
      "message": {
        "type": "string",
        "maxLength": 100,
        "position": 0
      }
    },
    "required": ["message"],
    "additionalProperties": false
  }
}`
- `groups` (json, optional) - Groups (optional)
  - Example: `{}`
- `tokens` (json, optional) - Tokens (optional)
  - Example: `{}`
- `keywords` (text, optional) - Keywords (comma separated, optional)
- `description` (text, optional) - Description (optional)

Example:
```javascript
const result = await sdk.dataContractCreate(identityHex, /* params */, privateKeyHex);
```

**Data Contract Update** - `dataContractUpdate`
*Add document types, groups, or tokens to an existing data contract*

Parameters (in addition to identity/key):
- `dataContractId` (text, required) - Data Contract ID
- `newDocumentSchemas` (json, optional) - New Document Schemas to Add (optional)
  - Example: `{
  "newType": {
    "type": "object",
    "properties": {
      "field": {
        "type": "string",
        "maxLength": 100,
        "position": 0
      }
    },
    "required": ["field"],
    "additionalProperties": false
  }
}`
- `newGroups` (json, optional) - New Groups to Add (optional)
  - Example: `{}`
- `newTokens` (json, optional) - New Tokens to Add (optional)
  - Example: `{}`

Example:
```javascript
const result = await sdk.dataContractUpdate(identityHex, /* params */, privateKeyHex);
```

#### Document Transitions

**Document Create** - `documentCreate`
*Create a new document*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `fetchSchema` (button, optional) - Fetch Schema
- `documentFields` (dynamic, optional) - Document Fields

Example:
```javascript
const result = await sdk.document_create(
    identityHex,
    contractId,
    "note",
    JSON.stringify({ message: "Hello!" }),
    privateKeyHex
);
```

**Document Replace** - `documentReplace`
*Replace an existing document*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `documentId` (text, required) - Document ID
- `loadDocument` (button, optional) - Load Document
- `documentFields` (dynamic, optional) - Document Fields

Example:
```javascript
const result = await sdk.documentReplace(identityHex, /* params */, privateKeyHex);
```

**Document Delete** - `documentDelete`
*Delete an existing document*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `documentId` (text, required) - Document ID

Example:
```javascript
const result = await sdk.documentDelete(identityHex, /* params */, privateKeyHex);
```

**Document Transfer** - `documentTransfer`
*Transfer document ownership*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `documentId` (text, required) - Document ID
- `recipientId` (text, required) - Recipient Identity ID

Example:
```javascript
const result = await sdk.documentTransfer(identityHex, /* params */, privateKeyHex);
```

**Document Purchase** - `documentPurchase`
*Purchase a document*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `documentId` (text, required) - Document ID
- `price` (number, required) - Price (credits)

Example:
```javascript
const result = await sdk.documentPurchase(identityHex, /* params */, privateKeyHex);
```

**Document Set Price** - `documentSetPrice`
*Set or update document price*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `documentType` (text, required) - Document Type
- `documentId` (text, required) - Document ID
- `price` (number, required) - Price (credits, 0 to remove)

Example:
```javascript
const result = await sdk.documentSetPrice(identityHex, /* params */, privateKeyHex);
```

**DPNS Register Name** - `dpnsRegister`
*Register a new DPNS username*

Parameters (in addition to identity/key):
- `label` (text, required) - Username
  - Example: `Enter username (e.g., alice)`

Example:
```javascript
const result = await sdk.dpnsRegister(identityHex, /* params */, privateKeyHex);
```

#### Token Transitions

**Token Burn** - `tokenBurn`
*Burn tokens*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `amount` (text, required) - Amount to Burn
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenBurn(identityHex, /* params */, privateKeyHex);
```

**Token Mint** - `tokenMint`
*Mint new tokens*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `amount` (text, required) - Amount to Mint
- `issuedToIdentityId` (text, optional) - Issue To Identity ID
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenMint(identityHex, /* params */, privateKeyHex);
```

**Token Claim** - `tokenClaim`
*Claim tokens from a distribution*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `distributionType` (select, required) - Distribution Type
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenClaim(identityHex, /* params */, privateKeyHex);
```

**Token Set Price** - `tokenSetPriceForDirectPurchase`
*Set or update the price for direct token purchases*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `priceType` (select, required) - Price Type
- `priceData` (text, optional) - Price Data (single price or JSON map)
  - Example: `Leave empty to remove pricing`
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenSetPriceForDirectPurchase(identityHex, /* params */, privateKeyHex);
```

**Token Direct Purchase** - `tokenDirectPurchase`
*Purchase tokens directly at the configured price*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `amount` (text, required) - Amount to Purchase
- `totalAgreedPrice` (text, optional) - Total Agreed Price (in credits) - Optional, fetches from pricing schedule if not provided

Example:
```javascript
const result = await sdk.tokenDirectPurchase(identityHex, /* params */, privateKeyHex);
```

**Token Config Update** - `tokenConfigUpdate`
*Update token configuration settings*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `configItemType` (select, required) - Config Item Type
- `configValue` (text, required) - Config Value (JSON or specific value)
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenConfigUpdate(identityHex, /* params */, privateKeyHex);
```

**Token Transfer** - `tokenTransfer`
*Transfer tokens between identities*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `amount` (text, required) - Amount to Transfer
- `recipientId` (text, required) - Recipient Identity ID
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.token_transfer(
    identityHex,
    contractId,
    tokenId,
    1000000, // amount
    recipientId,
    privateKeyHex
);
```

**Token Freeze** - `tokenFreeze`
*Freeze tokens for a specific identity*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `identityToFreeze` (text, required) - Identity ID to Freeze
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenFreeze(identityHex, /* params */, privateKeyHex);
```

**Token Unfreeze** - `tokenUnfreeze`
*Unfreeze tokens for a specific identity*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `identityToUnfreeze` (text, required) - Identity ID to Unfreeze
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenUnfreeze(identityHex, /* params */, privateKeyHex);
```

**Token Destroy Frozen** - `tokenDestroyFrozen`
*Destroy frozen tokens*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
- `tokenPosition` (number, required) - Token Contract Position
- `frozenIdentityId` (text, required) - Identity ID whose frozen tokens to destroy
- `publicNote` (text, optional) - Public Note

Example:
```javascript
const result = await sdk.tokenDestroyFrozen(identityHex, /* params */, privateKeyHex);
```

#### Voting Transitions

**DPNS Username** - `dpnsUsername`
*Cast a vote for a contested DPNS username*

Parameters (in addition to identity/key):
- `contestedUsername` (text, required) - Contested Username
  - Example: `Enter the contested username (e.g., 'myusername')`
- `voteChoice` (select, required) - Vote Choice
- `targetIdentity` (text, optional) - Target Identity ID (if voting for identity)
  - Example: `Identity ID to vote for`

Example:
```javascript
const result = await sdk.dpnsUsername(identityHex, /* params */, privateKeyHex);
```

**Contested Resource** - `masternodeVote`
*Cast a vote for contested resources as a masternode*

Parameters (in addition to identity/key):
- `contractId` (text, required) - Data Contract ID
  - Example: `Contract ID containing the contested resource`
- `fetchContestedResources` (button, optional) - Get Contested Resources
- `contestedResourceDropdown` (dynamic, optional) - Contested Resources
- `voteChoice` (select, required) - Vote Choice
- `targetIdentity` (text, optional) - Target Identity ID (if voting for identity)
  - Example: `Identity ID to vote for`

Example:
```javascript
const result = await sdk.masternodeVote(identityHex, /* params */, privateKeyHex);
```

## Common Patterns

### Error Handling
```javascript
try {
    const result = await sdk.getIdentity(identityId);
    console.log(result);
} catch (error) {
    console.error("Query failed:", error);
}
```

### Working with Proofs
```javascript
// Enable proofs during SDK initialization
const sdk = await WasmSdk.new(transport, true);

// Query with proof verification
const identityWithProof = await sdk.getIdentity(identityId);
```

### Document Queries with Where/OrderBy
```javascript
// Where clause format: [[field, operator, value], ...]
const whereClause = JSON.stringify([
    ["$ownerId", "==", identityId],
    ["age", ">=", 18]
]);

// OrderBy format: [[field, direction], ...]
const orderBy = JSON.stringify([
    ["$createdAt", "desc"]
]);

const docs = await sdk.getDocuments(
    contractId,
    documentType,
    whereClause,
    orderBy,
    limit
);
```

### Batch Operations
```javascript
// Get multiple identities
const identityIds = ["id1", "id2", "id3"];
const balances = await sdk.getIdentitiesBalances(identityIds);
```

## Important Notes

1. **Network Endpoints**: 
   - Testnet: `https://52.12.176.90:1443/`
   - Mainnet: Update when available

2. **Identity Format**: Identity IDs and keys should be hex-encoded strings

3. **Credits**: All fees are paid in credits (1 credit = 1 satoshi equivalent)

4. **Nonces**: The SDK automatically handles nonce management for state transitions

5. **Proofs**: Enable proofs for production applications to ensure data integrity

## Troubleshooting

- **Connection errors**: Verify network endpoint and that SDK is initialized
- **Invalid parameters**: Check parameter types and required fields
- **Authentication failures**: Ensure correct identity/key format and key permissions
- **Query errors**: Validate contract IDs, document types, and field names exist
