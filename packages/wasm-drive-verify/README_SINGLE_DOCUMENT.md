# Single Document Drive Query WASM Bindings

This module provides WASM bindings for verifying single document proofs from Dash Platform Drive.

## Overview

The `SingleDocumentDriveQuery` struct allows you to query and verify proofs for individual documents stored in Drive. This is useful when you need to verify that a specific document exists (or doesn't exist) at a particular point in time.

## Usage

### Creating a Query

```javascript
import { createSingleDocumentQuery, SingleDocumentDriveQueryWasm } from 'wasm-drive-verify';

// For a non-contested document
const query = createSingleDocumentQuery(
  contractId,        // Uint8Array(32) - The data contract ID
  'myDocumentType',  // string - The document type name
  false,             // boolean - Whether the document type keeps history
  documentId,        // Uint8Array(32) - The document ID
  Date.now()         // number (optional) - Block time in milliseconds
);

// For a maybe contested document
const queryMaybeContested = createSingleDocumentQueryMaybeContested(
  contractId,
  'myDocumentType',
  false,
  documentId,
  Date.now()
);

// For a contested document
const queryContested = createSingleDocumentQueryContested(
  contractId,
  'myDocumentType',
  false,
  documentId,
  Date.now()
);
```

### Verifying a Proof

```javascript
import { verifySingleDocumentProofKeepSerialized } from 'wasm-drive-verify';

// Verify a proof
const result = verifySingleDocumentProofKeepSerialized(
  query,      // SingleDocumentDriveQueryWasm - The query object
  false,      // boolean - Whether this is a subset of a larger proof
  proof       // Uint8Array - The proof to verify
);

// Access the results
console.log('Root hash:', result.rootHash);
console.log('Has document:', result.hasDocument());

if (result.hasDocument()) {
  console.log('Document (serialized):', result.documentSerialized);
}
```

## Contested Status

Documents in Drive can have different contested statuses:

- **NotContested (0)**: The document was not contested by the system
- **MaybeContested (1)**: We don't know if the document was contested, or we're not sure if the contest is over
- **Contested (2)**: We know that the document was contested and the contest is not over

## API Reference

### Classes

#### SingleDocumentDriveQueryWasm

Represents a query for a single document in Drive.

**Constructor:**
- `contractId: Uint8Array` - The contract ID (must be exactly 32 bytes)
- `documentTypeName: string` - The name of the document type
- `documentTypeKeepsHistory: boolean` - Whether the document type keeps history
- `documentId: Uint8Array` - The document ID (must be exactly 32 bytes)
- `blockTimeMs?: number` - Optional block time in milliseconds
- `contestedStatus?: number` - The contested status (0, 1, or 2)

**Properties:**
- `contractId: Uint8Array` - The contract ID
- `documentTypeName: string` - The document type name
- `documentTypeKeepsHistory: boolean` - Whether the document type keeps history
- `documentId: Uint8Array` - The document ID
- `blockTimeMs?: number` - The block time in milliseconds
- `contestedStatus: number` - The contested status

#### SingleDocumentProofResult

The result of a single document proof verification.

**Properties:**
- `rootHash: Uint8Array` - The root hash of the proof
- `documentSerialized?: Uint8Array` - The serialized document (if found)

**Methods:**
- `hasDocument(): boolean` - Check if a document was found

### Functions

#### verifySingleDocumentProofKeepSerialized

Verifies a single document proof and keeps the document serialized.

**Parameters:**
- `query: SingleDocumentDriveQueryWasm` - The query to verify
- `isSubset: boolean` - Whether to verify a subset of a larger proof
- `proof: Uint8Array` - The proof to verify

**Returns:** `SingleDocumentProofResult`

#### createSingleDocumentQuery

Creates a query for a non-contested document.

**Parameters:**
- `contractId: Uint8Array` - The contract ID (must be exactly 32 bytes)
- `documentTypeName: string` - The name of the document type
- `documentTypeKeepsHistory: boolean` - Whether the document type keeps history
- `documentId: Uint8Array` - The document ID (must be exactly 32 bytes)
- `blockTimeMs?: number` - Optional block time in milliseconds

**Returns:** `SingleDocumentDriveQueryWasm`

#### createSingleDocumentQueryMaybeContested

Creates a query for a maybe contested document.

**Parameters:** Same as `createSingleDocumentQuery`

**Returns:** `SingleDocumentDriveQueryWasm`

#### createSingleDocumentQueryContested

Creates a query for a contested document.

**Parameters:** Same as `createSingleDocumentQuery`

**Returns:** `SingleDocumentDriveQueryWasm`

## Building

To build the WASM module:

```bash
cd packages/wasm-drive-verify
./scripts/build-wasm.sh
```

This will generate the WASM files in the `wasm/` directory.