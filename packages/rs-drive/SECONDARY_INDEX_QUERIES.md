# Dash Platform Secondary Index Document Queries - Comprehensive Guide for AI Agents

## Overview

Dash Platform uses a hierarchical authenticated data structure based on GroveDB (an AVL merkle forest) to store and query documents. This guide explains the query system capabilities, limitations, and implementation details for secondary index queries.

## Key Concepts

### 1. Query Structure

All document queries in Dash Platform are represented by the `DriveDocumentQuery` struct, which contains:

```rust
pub struct DriveDocumentQuery<'a> {
    pub contract: &'a DataContract,
    pub document_type: DocumentTypeRef<'a>,
    pub internal_clauses: InternalClauses,
    pub offset: Option<u16>,
    pub limit: Option<u16>,
    pub order_by: IndexMap<String, OrderClause>,
    pub start_at: Option<[u8; 32]>,
    pub start_at_included: bool,
    pub block_time_ms: Option<u64>,
}
```

### 2. Internal Clauses

The `InternalClauses` struct organizes WHERE clauses into categories:

```rust
pub struct InternalClauses {
    pub primary_key_in_clause: Option<WhereClause>,      // IN clause on $id
    pub primary_key_equal_clause: Option<WhereClause>,   // == clause on $id
    pub in_clause: Option<WhereClause>,                  // Single IN clause on indexed field
    pub range_clause: Option<WhereClause>,               // Single range clause (may be combined from 2)
    pub equal_clauses: BTreeMap<String, WhereClause>,    // Multiple == clauses
}
```

## Query Capabilities

### 1. Supported Operators

- **Equality**: `==` (Equal)
- **Comparison**: `>` (GreaterThan), `>=` (GreaterThanOrEquals), `<` (LessThan), `<=` (LessThanOrEquals)
- **Range**: `Between`, `BetweenExcludeBounds`, `BetweenExcludeLeft`, `BetweenExcludeRight`
- **Membership**: `in` (In)
- **String**: `StartsWith`

### 2. Multiple Range Queries on Single Field

**YES, but with strict limitations:**

- **Exactly 2 range clauses** can be combined on the **same field**
- They must form valid bounds (one upper, one lower)
- Valid combinations are automatically converted to Between operators:
  - `field >= X && field <= Y` → `field Between [X, Y]`
  - `field >= X && field < Y` → `field BetweenExcludeRight [X, Y]`
  - `field > X && field <= Y` → `field BetweenExcludeLeft [X, Y]`
  - `field > X && field < Y` → `field BetweenExcludeBounds [X, Y]`

**Example:**
```json
{
  "where": [
    ["age", ">", 18],
    ["age", "<", 65]
  ]
}
```
This is valid and will be internally converted to: `age BetweenExcludeBounds [18, 65]`

### 3. Query Types

#### Simple Equality Query
```json
{
  "where": [
    ["firstName", "==", "Alice"]
  ]
}
```

#### Range Query
```json
{
  "where": [
    ["age", ">=", 18]
  ]
}
```

#### IN Query
```json
{
  "where": [
    ["status", "in", ["active", "pending", "approved"]]
  ]
}
```

#### Combined Query
```json
{
  "where": [
    ["category", "==", "electronics"],
    ["price", ">=", 100],
    ["price", "<=", 1000]
  ],
  "orderBy": [
    ["price", "asc"]
  ],
  "limit": 50
}
```

### 4. Special Fields

These fields can be queried without being explicitly defined in the document type:
- `$id` - Document identifier
- `$ownerId` - Owner identifier
- `$createdAt` - Creation timestamp
- `$updatedAt` - Update timestamp
- `$revision` - Document revision number

## Compound Index Queries

### How Compound Indexes Work

When an index has multiple properties [A, B, C], the query system creates a hierarchical tree structure:
- Level 1: Property A values
- Level 2: Property B values (under each A value)
- Level 3: Property C values (under each A,B combination)
- Level 4: Document IDs

### Range Queries on Compound Indexes

**Range queries CAN be performed on any property in a compound index**, but you MUST provide equality constraints for ALL properties that come before it in the index.

#### For index [A, B, C]:

✅ **Valid - Range on A (first property):**
```json
{
  "where": [
    ["A", ">", 10],
    ["A", "<", 20]
  ]
}
```

✅ **Valid - Range on B with equality on A:**
```json
{
  "where": [
    ["A", "==", 5],      // Required: equality on A
    ["B", ">", 100],
    ["B", "<=", 200]
  ]
}
```

✅ **Valid - Range on C with equalities on A and B:**
```json
{
  "where": [
    ["A", "==", 5],      // Required: equality on A
    ["B", "==", 10],     // Required: equality on B
    ["C", ">=", 1000]
  ]
}
```

❌ **Invalid - Range on B without equality on A:**
```json
{
  "where": [
    ["B", ">", 100]      // Error: Cannot query B without specifying A
  ]
}
```

❌ **Invalid - Range on C without equality on B:**
```json
{
  "where": [
    ["A", "==", 5],
    ["C", ">", 1000]     // Error: Cannot skip B in the index
  ]
}
```

### Why This Design?

The hierarchical structure means:
1. To query at any level, you must provide the complete path to that level
2. This is like a file system where you can't list `/home/*/documents/` without specifying which home directory
3. It ensures efficient traversal of the authenticated merkle tree structure

### Practical Example

Given an index on [category, brand, price]:
```json
// Query electronics from Apple with price range
{
  "where": [
    ["category", "==", "electronics"],  // Required
    ["brand", "==", "Apple"],          // Required
    ["price", ">=", 500],
    ["price", "<=", 2000]
  ]
}
```

This creates the path: `/category=electronics/brand=Apple/price=[500,2000]/`

## Restrictions and Validation Rules

### 1. Index Requirements

- **Queries MUST match an index** defined in the document type
- Non-indexed fields cannot be queried (error: `WhereClauseOnNonIndexedProperty`)
- Query must be within `MAX_INDEX_DIFFERENCE` (2) of an existing index

### 2. Limit Restrictions

- **Default limit**: 100 documents
- **Maximum limit**: 100 documents
- **Absolute maximum**: 65,535 (u16::MAX)
- If limit is 0, defaults to system default (100)

### 3. Clause Restrictions

- **Primary key queries**: Only ONE equal or IN clause allowed on `$id`
- **IN clause limits**:
  - Minimum: 1 value
  - Maximum: 100 values
  - No duplicate values
- **Range clauses**: Maximum 2 on the same field (combined into Between)
- **One IN clause per query**: Cannot have multiple IN clauses
- **Field overlap**: Same field cannot appear in different clause types

### 4. Invalid Query Examples

#### Multiple IN clauses (INVALID)
```json
{
  "where": [
    ["category", "in", ["A", "B"]],
    ["status", "in", ["active", "pending"]]
  ]
}
```

#### Range on multiple fields (INVALID)
```json
{
  "where": [
    ["age", ">", 18],
    ["salary", "<", 100000]
  ]
}
```

#### Conflicting clauses (INVALID)
```json
{
  "where": [
    ["age", "==", 25],
    ["age", ">", 30]
  ]
}
```

## ProveOptions and GroveDB Integration

### ProveOptions Structure

The `ProveOptions` struct (from grovedb v3.0.0) configures proof generation:

```rust
pub struct ProveOptions {
    /// Decrease the available limit by 1 for empty subtrees
    /// WARNING: Set to false only if you know there are few empty subtrees
    /// Otherwise can cause memory exhaustion
    pub decrease_limit_on_empty_sub_query_result: bool,
}
```

### Key Points:

1. **Default behavior**: When `None` is passed (common in codebase), default options are used
2. **Memory safety**: The `decrease_limit_on_empty_sub_query_result` flag prevents memory exhaustion when traversing many empty branches in the merkle tree
3. **Usage**: Pass to `grovedb.get_proved_path_query()` as the second parameter

## Query Execution Flow

1. **Parsing**: JSON/CBOR query → `DocumentQuery` struct
2. **Validation**: 
   - Check field existence
   - Validate operators
   - Ensure index match
   - Check limits
3. **Internal Clause Extraction**: Group clauses into `InternalClauses`
4. **Index Selection**: Find best matching index
5. **Path Query Generation**: Convert to GroveDB `PathQuery`
6. **Execution**: 
   - Without proof: Direct query execution
   - With proof: Use `get_proved_path_query()` with optional `ProveOptions`
7. **Result**: Documents with optional cryptographic proof

## Best Practices for AI Agents

1. **Always check if fields are indexed** before querying
2. **Use compound indexes** when querying multiple fields
3. **Prefer equality queries** over range queries for performance
4. **Limit IN clause values** to reasonable amounts (<50 recommended)
5. **Use pagination** (start_at/start_after) for large result sets
6. **Combine range clauses** on same field for efficiency
7. **Avoid queries far from indexes** (will be rejected)

## Error Handling

Common query errors:
- `QuerySyntaxError::WhereClauseOnNonIndexedProperty` - Field not indexed
- `QuerySyntaxError::InvalidInClauseValue` - IN clause validation failed
- `QuerySyntaxError::MultipleRangeClauses` - Invalid range combination
- `QuerySyntaxError::QueryTooFarFromIndex` - No suitable index found
- `QuerySyntaxError::InvalidLimit` - Limit exceeds maximum

## Summary

Dash Platform's query system is designed for:
- **Efficiency**: Requires indexed fields
- **Safety**: Limits prevent resource exhaustion
- **Flexibility**: Supports various operators and combinations
- **Verifiability**: Optional cryptographic proofs via GroveDB

The system intelligently combines multiple range clauses on the same field but restricts complex multi-field range queries to ensure performance and predictability in the distributed environment.