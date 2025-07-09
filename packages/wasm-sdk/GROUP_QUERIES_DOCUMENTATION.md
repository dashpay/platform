# Group Queries Implementation

This document describes the implementation of group queries in the WASM SDK.

## Overview

The group queries allow you to interact with group functionality in Dash Platform data contracts. Groups are used to manage collective ownership and permissions.

## Implemented Queries

### 1. `get_group_info`

Fetches information about a specific group.

**Parameters:**
- `sdk`: The WASM SDK instance
- `data_contract_id`: The data contract ID (Base58 encoded string)
- `group_contract_position`: The position of the group in the contract (u32)

**Returns:**
```javascript
{
  "members": {
    "identityId1": 100,  // member ID -> voting power
    "identityId2": 50
  },
  "requiredPower": 100  // minimum power needed for decisions
}
```
Returns `null` if the group doesn't exist.

### 2. `get_group_members`

Gets members of a specific group with optional filtering and pagination.

**Parameters:**
- `sdk`: The WASM SDK instance
- `data_contract_id`: The data contract ID (Base58 encoded string)
- `group_contract_position`: The position of the group in the contract (u32)
- `member_ids`: Optional array of specific member IDs to fetch
- `start_at`: Optional member ID to start pagination from
- `limit`: Optional limit on number of results

**Returns:**
```javascript
[
  {
    "memberId": "identityId1",
    "power": 100
  },
  {
    "memberId": "identityId2",
    "power": 50
  }
]
```

### 3. `get_identity_groups`

Retrieves all groups associated with a specific identity.

**Parameters:**
- `sdk`: The WASM SDK instance
- `identity_id`: The identity ID to search for (Base58 encoded string)
- `member_data_contracts`: Optional array of contract IDs to search for member roles
- `owner_data_contracts`: Optional array of contract IDs to search for owner roles (not yet implemented)
- `moderator_data_contracts`: Optional array of contract IDs to search for moderator roles (not yet implemented)

**Returns:**
```javascript
[
  {
    "dataContractId": "contractId1",
    "groupContractPosition": 0,
    "role": "member",
    "power": 100  // only for member role
  }
]
```

**Note:** Currently only member role queries are implemented. Owner and moderator roles require additional contract queries not yet available in the SDK.

### 4. `get_groups_data_contracts`

Fetches all groups for multiple data contracts.

**Parameters:**
- `sdk`: The WASM SDK instance
- `data_contract_ids`: Array of data contract IDs to fetch groups from

**Returns:**
```javascript
[
  {
    "dataContractId": "contractId1",
    "groups": [
      {
        "position": 0,
        "group": {
          "members": {
            "identityId1": 100
          },
          "requiredPower": 100
        }
      }
    ]
  }
]
```

## Implementation Details

The implementation uses:
- Dash SDK's `Fetch` and `FetchMany` traits for querying
- `GroupQuery` and `GroupInfosQuery` types from the SDK
- `serde_wasm_bindgen` with `json_compatible` serializer for proper JavaScript object conversion
- Base58 encoding for all identifiers passed to/from JavaScript

## Error Handling

All functions return proper error messages when:
- Invalid identifiers are provided
- Network errors occur
- Groups or contracts don't exist

## Example Usage

```javascript
import init, { 
    WasmSdkBuilder, 
    get_group_info,
    get_group_members,
    get_identity_groups,
    get_groups_data_contracts
} from './pkg/wasm_sdk.js';

// Initialize SDK
await init();
const builder = new WasmSdkBuilder();
builder.with_core("127.0.0.1", 20002, "regtest", "");
const sdk = await builder.build();

// Get group info
const groupInfo = await get_group_info(
    sdk, 
    'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    0
);

// Get group members with pagination
const members = await get_group_members(
    sdk,
    'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
    0,
    null,  // all members
    null,  // start from beginning
    10     // limit to 10 results
);

// Get groups for an identity
const identityGroups = await get_identity_groups(
    sdk,
    '4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF',
    ['GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'], // check member role in this contract
    null,
    null
);
```