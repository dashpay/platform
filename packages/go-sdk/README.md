# Dash Platform Go SDK

The official Go SDK for interacting with Dash Platform, built on top of the Rust FFI bindings (rs-sdk-ffi). This SDK provides a high-level, idiomatic Go interface to the Dash decentralized application platform.

## Features

- **Identity Management**: Create identities, manage keys, transfer credits
- **Data Contracts**: Deploy and manage data contracts with schema validation
- **Document Operations**: Full CRUD operations with advanced querying
- **Token Operations**: Mint, burn, and transfer tokens
- **Advanced Queries**: Fluent query builder with support for complex conditions
- **Type Safety**: Strong typing with custom types for identifiers
- **Memory Safety**: Automatic memory management with finalizers
- **Context Support**: Full support for Go contexts and cancellation
- **Comprehensive Error Handling**: Detailed error types with wrapping

## Installation

```bash
go get github.com/dashpay/platform/packages/go-sdk
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"
    
    dash "github.com/dashpay/platform/packages/go-sdk"
)

func main() {
    // Create SDK configuration
    config := &dash.Config{
        Network: dash.NetworkTestnet,
        GRPCHost: "seed-1.testnet.networks.dash.org",
        GRPCPort: 1443,
    }
    
    // Initialize SDK
    sdk, err := dash.NewSDK(config)
    if err != nil {
        log.Fatal(err)
    }
    defer sdk.Close()
    
    // Create an identity
    identity, err := sdk.Identities().Create()
    if err != nil {
        log.Fatal(err)
    }
    
    fmt.Printf("Created identity: %s\n", identity.GetID())
}
```

## Detailed Usage

### Identity Management

```go
// Create a new identity
identity, err := sdk.Identities().Create()

// Fetch an existing identity
identity, err := sdk.Identities().Get(identityID)

// Get identity balance
balance, err := identity.GetBalance()

// Transfer credits
err = identity.TransferCredits(ctx, recipientID, amount)

// Top up identity
err = identity.TopUp(ctx, amount)

// Get many identities at once
identities, err := sdk.Identities().GetMany([]IdentityID{id1, id2, id3})

// Get identity balances for multiple identities
balances, err := sdk.Identities().GetBalances([]IdentityID{id1, id2})
```

### Data Contracts

```go
// Create a data contract
contractDef := `{
    "myDocument": {
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "email": {"type": "string", "format": "email"}
        },
        "required": ["name"],
        "additionalProperties": false
    }
}`

contract, err := sdk.Contracts().Create(identity, contractDef)

// Fetch a contract
contract, err := sdk.Contracts().Get(contractID)

// Put contract on platform
err = contract.Put(ctx, identity, &dash.PutSettings{
    MinGasFees: 1000,
})

// Get contract schema
schema, err := contract.GetSchema("myDocument")

// Create documents from contract
doc, err := contract.CreateDocument("myDocument", identity, map[string]interface{}{
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com",
})
```

### Document Operations

```go
// Create a document
doc, err := contract.CreateDocument("myDocument", identity, map[string]interface{}{
    "name": "Alice",
    "age": 30,
})

// Update document properties - multiple methods available
err = doc.Set("name", "Bob")                              // Simple field update
err = doc.SetProperty("address.city", "Boston")           // Nested field with path
err = doc.RemoveProperty("age")                          // Remove optional field

// Put document on platform
err = doc.Put(ctx, identity, putSettings, nil)

// Replace document on platform (after updates)
err = doc.Replace(ctx, identity, putSettings)

// Delete document
err = doc.Delete(ctx, identity, putSettings)

// Transfer document ownership
err = doc.Transfer(ctx, newOwnerID, identity, putSettings)
```

### Advanced Querying

```go
// Build complex queries with fluent API
query := sdk.Documents().NewQuery(contract, "myDocument").
    Where("name").Equals("Alice").
    Where("age").GreaterThan(25).
    Where("age").LessThanOrEqual(35).
    Where("status").In([]string{"active", "pending"}).
    Where("tags").Contains("developer").
    OrderBy("age", false).  // false = descending
    StartAfter(lastDocID).  // For pagination
    Limit(20)

// Execute query
documents, err := query.Execute(ctx)

// Search with raw query (Platform Query Language)
results, err := sdk.Documents().Search(contract, "myDocument", `{
    "where": [
        ["age", ">=", 18],
        ["status", "==", "active"]
    ],
    "orderBy": [["createdAt", "desc"]],
    "limit": 100
}`)

// Fetch specific document
doc, err := sdk.Documents().Get(contractID, "myDocument", documentID)
```

### Token Operations

```go
// Mint tokens
err = sdk.Tokens().Mint(ctx, tokenID, identity, amount, recipientID)

// Burn tokens
err = sdk.Tokens().Burn(ctx, tokenID, identity, amount)

// Transfer tokens
err = sdk.Tokens().Transfer(ctx, tokenID, identity, amount, recipientID)

// Get token balance
balance, err := sdk.Tokens().GetBalance(tokenID, identityID)

// Get token info
info, err := sdk.Tokens().GetInfo(tokenID)

// Purchase document with tokens
err = doc.Purchase(ctx, buyerIdentity, tokenID, putSettings)
```

### Error Handling

```go
// SDK provides detailed error types
identity, err := sdk.Identities().Get(identityID)
if err != nil {
    switch {
    case errors.Is(err, dash.ErrNotFound):
        // Handle not found case
        fmt.Println("Identity not found")
    case errors.Is(err, dash.ErrNetworkError):
        // Handle network issues
        fmt.Println("Network error:", err)
    case errors.Is(err, dash.ErrInvalidParameter):
        // Handle invalid input
        fmt.Println("Invalid parameter:", err)
    default:
        // Handle other errors
        return fmt.Errorf("unexpected error: %w", err)
    }
}
```

### Memory Management

```go
// Automatic cleanup with finalizers (default)
identity, err := sdk.Identities().Create()
// Resources automatically freed when identity goes out of scope

// Manual cleanup for fine-grained control
identity, err := sdk.Identities().Create()
defer identity.Close() // Explicitly release resources

// SDK-level cleanup
sdk, err := dash.NewSDK(config)
defer sdk.Close() // Cleans up all SDK resources
```

### Testing Support

```go
func TestMyFunction(t *testing.T) {
    // Create a mock SDK for offline testing
    sdk := dash.NewMockSDK()
    defer sdk.Close()
    
    // Mock SDK returns predictable test data
    identity, err := sdk.Identities().Create()
    assert.NoError(t, err)
    assert.NotEmpty(t, identity.GetID())
}
```

## Architecture

The Go SDK architecture follows a modular design:

```
go-sdk/
├── sdk.go           # Main SDK entry point
├── identity.go      # Identity management
├── contract.go      # Data contract operations
├── document.go      # Document CRUD and queries
├── token.go         # Token operations
├── query.go         # Query builder
├── types.go         # Common types and interfaces
├── errors.go        # Error definitions
└── internal/
    └── ffi/         # CGO bindings to Rust
        ├── bindings.go
        └── memory.go
```

### Key Design Decisions

1. **Sub-module Pattern**: Clean API with `sdk.Identities()`, `sdk.Documents()`, etc.
2. **Builder Pattern**: Fluent query builder for complex queries
3. **Strong Typing**: Custom types for IDs prevent mixing different identifier types
4. **Context Support**: All network operations accept context for cancellation
5. **Error Wrapping**: Detailed error context with `fmt.Errorf` and custom error types

## Requirements

- Go 1.19 or higher (uses newer error handling features)
- CGO enabled (for FFI bindings)
- C compiler (gcc/clang)
- Dash Platform FFI library (rs-sdk-ffi)

## Building

1. Build the Rust FFI library:
```bash
cd ../rs-sdk-ffi
cargo build --release
```

2. Build the Go SDK:
```bash
go build ./...
```

3. Run tests:
```bash
go test ./...
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

See the main platform repository for detailed contribution guidelines.

## License

MIT License - see LICENSE file in the root repository for details.