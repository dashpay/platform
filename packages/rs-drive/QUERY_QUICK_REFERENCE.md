# Dash Platform Query Quick Reference

## Can we do multiple ranges on one index?

**YES, with limitations:**
- ✅ **2 range clauses on SAME field** → Combined into Between operator
- ❌ **Range clauses on DIFFERENT fields** → Not supported
- ❌ **More than 2 range clauses** → Not supported

### Valid Example:
```json
{
  "where": [
    ["age", ">", 18],
    ["age", "<=", 65]
  ]
}
```
→ Internally becomes: `age BetweenExcludeLeft [18, 65]`

### Invalid Example:
```json
{
  "where": [
    ["age", ">", 18],
    ["salary", "<", 100000]
  ]
}
```
→ Error: Range clauses must be on same field

## Compound Index Queries

For index [A, B, C], ranges can be on ANY property BUT:
- Range on A: No requirements
- Range on B: MUST have `A == value`
- Range on C: MUST have `A == value` AND `B == value`

### Examples:
```json
// ✅ Valid: Range on B with equality on A
{
  "where": [
    ["A", "==", 5],
    ["B", ">", 10]
  ]
}

// ❌ Invalid: Range on B without equality on A
{
  "where": [
    ["B", ">", 10]  // Error: Missing A
  ]
}
```

## Key Query Rules

1. **Fields MUST be indexed** (no queries on non-indexed fields)
2. **One IN clause per query** (max 100 values)
3. **One range clause per query** (can be combined from 2 on same field)
4. **Limits**: Default 100, Max 100 (configurable)
5. **Special queryable fields**: `$id`, `$ownerId`, `$createdAt`, `$updatedAt`, `$revision`

## ProveOptions Warning

When using proof generation with GroveDB:
```rust
ProveOptions {
    decrease_limit_on_empty_sub_query_result: true, // ALWAYS use true in production
}
```
Setting to `false` can cause memory exhaustion if query matches many empty subtrees!