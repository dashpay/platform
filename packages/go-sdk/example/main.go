package main

import (
	"context"
	"fmt"
	"log"

	dash "github.com/dashpay/platform/packages/go-sdk"
)

func main() {
	// Print SDK version
	fmt.Printf("Dash SDK Version: %s\n", dash.Version())

	// Create SDK instance for testnet
	sdk, err := dash.NewSDK(dash.ConfigTestnet())
	if err != nil {
		log.Fatalf("Failed to create SDK: %v", err)
	}
	defer sdk.Close()

	ctx := context.Background()

	// Get network
	network, err := sdk.GetNetwork()
	if err != nil {
		log.Fatalf("Failed to get network: %v", err)
	}
	fmt.Printf("Connected to network: %s\n", network)

	// Example: Fetch an identity (would fail without real identity ID)
	/*
	identity, err := sdk.Identities().Get(ctx, "someIdentityId")
	if err != nil {
		log.Printf("Failed to fetch identity: %v", err)
	} else {
		info, _ := identity.GetInfo()
		fmt.Printf("Identity ID: %s\n", info.ID)
		fmt.Printf("Balance: %d\n", info.Balance)
	}
	*/

	// Example: Create a new identity (would need funding)
	/*
	newIdentity, err := sdk.Identities().Create(ctx)
	if err != nil {
		log.Printf("Failed to create identity: %v", err)
	} else {
		id, _ := newIdentity.GetID()
		fmt.Printf("Created new identity: %s\n", id)
	}
	*/

	// Example: Create a data contract
	/*
	owner := ... // Get or create identity
	
	// Define a simple document type
	definitions := map[string]dash.DocumentTypeDefinition{
		"message": dash.CreateDocumentTypeDefinition(
			map[string]dash.PropertySchema{
				"text": dash.CreatePropertySchema("string", 
					dash.WithMaxLength(280),
					dash.WithDescription("Message text"),
				),
				"author": dash.CreatePropertySchema("string",
					dash.WithDescription("Author name"),
				),
				"timestamp": dash.CreatePropertySchema("integer",
					dash.WithDescription("Unix timestamp"),
				),
			},
			[]string{"text", "author", "timestamp"}, // required fields
			[]dash.Index{
				dash.CreateIndex("timestampIndex", 
					[]dash.IndexProperty{
						dash.CreateIndexProperty("timestamp", false), // descending
					},
					false, // not unique
					false, // not sparse
				),
			},
		),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	if err != nil {
		log.Printf("Failed to create contract: %v", err)
	}
	*/

	// Example: Create and query documents
	/*
	document, err := sdk.Documents().Create(ctx, dash.CreateParams{
		DataContract: contract,
		DocumentType: "message",
		Owner:        owner,
		Properties: map[string]interface{}{
			"text":      "Hello from Go SDK!",
			"author":    "Go Developer",
			"timestamp": time.Now().Unix(),
		},
	})
	if err != nil {
		log.Printf("Failed to create document: %v", err)
	}

	// Query documents
	query := dash.NewQueryBuilder().
		Where("author", "Go Developer").
		OrderBy("timestamp", false).
		Limit(10).
		Build()

	results, err := sdk.Documents().Search(ctx, contract, "message", query)
	if err != nil {
		log.Printf("Failed to search documents: %v", err)
	}
	*/

	fmt.Println("SDK example completed successfully")
}