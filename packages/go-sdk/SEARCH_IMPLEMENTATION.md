# Document Search Implementation

## Overview

The `Search()` function in the Go SDK has been properly implemented to create document objects from search results. This replaces the previous stub implementation that always returned an empty slice.

## Implementation Details

### Search Result Format

The FFI layer returns search results as a JSON string with the following structure:

```json
{
  "documents": [
    {
      "$id": "document_id",
      "$ownerId": "owner_identity_id",
      "$revision": 1,
      "$createdAt": 1234567890,
      "$updatedAt": 1234567890,
      // ... custom document fields
    }
  ],
  "total_count": 100
}
```

### Key Features

1. **Document Creation Without Handles**:
   - Search results return document data, not handles
   - Documents are created with `handle: nil` 
   - These documents can be used for reading but not for write operations

2. **Metadata Extraction**:
   - System fields (prefixed with `$`) are extracted: `$id`, `$ownerId`, `$revision`, `$createdAt`, `$updatedAt`
   - Custom document fields are preserved in the `Data` map
   - Contract ID is set from the provided contract parameter

3. **Handle-Required Operations**:
   - Added `HasHandle()` method to check if a document can be used for writes
   - Added `requireHandle()` helper that returns descriptive errors
   - All write operations (Put, Replace, Delete, Transfer, etc.) check for handle

4. **Number Field Handling**:
   - Added `getNumberField()` helper to handle various JSON number types
   - Supports float64, float32, int, int64, uint64, and json.Number

## Usage Example

```go
// Search for documents
query := NewQueryBuilder().
    Where("author", "alice").
    WhereGT("timestamp", 1000000000).
    OrderBy("timestamp", false).
    Limit(10).
    Build()

results, err := sdk.Documents().Search(ctx, contract, "message", query)
if err != nil {
    log.Fatal(err)
}

for _, doc := range results {
    // Read document data
    data, _ := doc.GetData()
    fmt.Printf("Message: %v\n", data["text"])
    
    // Check if document has a handle
    if !doc.HasHandle() {
        fmt.Println("Document from search - read-only")
    }
    
    // This would fail with appropriate error
    err = doc.Put(ctx, identity, nil, nil)
    if err != nil {
        fmt.Println("Expected error:", err)
        // Output: cannot put document without handle - document was created from search results
    }
    
    doc.Close()
}
```

## Document Lifecycle

1. **Documents from Create()**: Have handles, can perform all operations
2. **Documents from Get()**: Have handles, can perform all operations  
3. **Documents from Search()**: No handles, read-only access to data

## Error Messages

When attempting write operations on search result documents:
- Put: "cannot put document without handle - document was created from search results"
- Replace: "cannot replace document without handle - document was created from search results"
- Delete: "cannot delete document without handle - document was created from search results"
- Transfer: "cannot transfer document without handle - document was created from search results"
- Purchase: "cannot purchase document without handle - document was created from search results"
- UpdatePrice: "cannot update price document without handle - document was created from search results"
- Destroy: "cannot destroy document without handle - document was created from search results"

## Testing

Comprehensive tests verify:
- Document objects are created from search results
- Documents have no handles and return false for `HasHandle()`
- Document data is accessible via `GetData()` and `GetInfo()`
- Write operations fail with appropriate error messages
- Complex queries work correctly
- Empty results are handled gracefully

## Technical Notes

1. The `info` field is pre-populated for search results to avoid calling FFI with nil handle
2. Standard document fields use the `$` prefix convention
3. The `total_count` field from the response could be used for pagination (currently not exposed)
4. Search results are ideal for display/read scenarios where handles aren't needed

This implementation provides a clean separation between read-only search results and full document objects, preventing accidental misuse while maintaining a consistent API.