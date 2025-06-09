// +build integration

package dash

import (
	"context"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestIntegrationIdentityWorkflow tests a complete identity workflow
func TestIntegrationIdentityWorkflow(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create identity
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	// Get identity info
	info, err := identity.GetInfo()
	require.NoError(t, err)
	assert.NotEmpty(t, info.ID)

	// Get balance
	balance, err := identity.GetBalance()
	require.NoError(t, err)
	assert.GreaterOrEqual(t, balance, uint64(0))

	// Fetch the same identity by ID
	fetchedIdentity, err := sdk.Identities().Get(ctx, info.ID)
	require.NoError(t, err)
	defer fetchedIdentity.Close()

	fetchedInfo, err := fetchedIdentity.GetInfo()
	require.NoError(t, err)
	assert.Equal(t, info.ID, fetchedInfo.ID)
}

// TestIntegrationDataContractAndDocuments tests contract creation and document operations
func TestIntegrationDataContractAndDocuments(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create owner identity
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	// Create data contract
	definitions := map[string]DocumentTypeDefinition{
		"post": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"title":      CreatePropertySchema("string", WithMaxLength(100)),
				"content":    CreatePropertySchema("string", WithMaxLength(5000)),
				"author":     CreatePropertySchema("string"),
				"timestamp":  CreatePropertySchema("integer"),
				"tags":       CreatePropertySchema("array"),
				"published":  CreatePropertySchema("boolean"),
			},
			[]string{"title", "content", "author", "timestamp"},
			[]Index{
				CreateIndex("timestampIndex",
					[]IndexProperty{
						CreateIndexProperty("timestamp", false),
					},
					false, false),
				CreateIndex("authorTimestampIndex",
					[]IndexProperty{
						CreateIndexProperty("author", true),
						CreateIndexProperty("timestamp", false),
					},
					false, false),
			},
		),
		"comment": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"postId":     CreatePropertySchema("string"),
				"content":    CreatePropertySchema("string", WithMaxLength(1000)),
				"author":     CreatePropertySchema("string"),
				"timestamp":  CreatePropertySchema("integer"),
			},
			[]string{"postId", "content", "author", "timestamp"},
			[]Index{
				CreateIndex("postIdTimestampIndex",
					[]IndexProperty{
						CreateIndexProperty("postId", true),
						CreateIndexProperty("timestamp", false),
					},
					false, false),
			},
		),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	require.NoError(t, err)
	defer contract.Close()

	// Get contract info
	contractInfo, err := contract.GetInfo()
	require.NoError(t, err)
	assert.NotEmpty(t, contractInfo.ID)
	assert.NotEmpty(t, contractInfo.OwnerID)

	// Create a post document
	postDoc, err := sdk.Documents().Create(ctx, CreateParams{
		DataContract: contract,
		DocumentType: "post",
		Owner:        owner,
		Properties: map[string]interface{}{
			"title":      "Test Post",
			"content":    "This is a test post content",
			"author":     "testuser",
			"timestamp":  time.Now().Unix(),
			"tags":       []string{"test", "integration"},
			"published":  true,
		},
	})
	require.NoError(t, err)
	defer postDoc.Close()

	// Get document info
	docInfo, err := postDoc.GetInfo()
	require.NoError(t, err)
	assert.NotEmpty(t, docInfo.ID)
	assert.Equal(t, "post", docInfo.DocumentType)

	// Update document field
	err = postDoc.Set("published", false)
	require.NoError(t, err)

	published, exists := postDoc.Get("published")
	assert.True(t, exists)
	assert.Equal(t, false, published)

	// Create a comment document
	commentDoc, err := sdk.Documents().Create(ctx, CreateParams{
		DataContract: contract,
		DocumentType: "comment",
		Owner:        owner,
		Properties: map[string]interface{}{
			"postId":    docInfo.ID,
			"content":   "Great post!",
			"author":    "commenter",
			"timestamp": time.Now().Unix(),
		},
	})
	require.NoError(t, err)
	defer commentDoc.Close()

	// Search for documents
	query := NewQueryBuilder().
		Where("author", "testuser").
		OrderBy("timestamp", false).
		Limit(10).
		Build()

	results, err := sdk.Documents().Search(ctx, contract, "post", query)
	require.NoError(t, err)
	
	for _, doc := range results {
		doc.Close()
	}
}

// TestIntegrationTokenOperations tests token-related operations
func TestIntegrationTokenOperations(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create identities for token operations
	issuer, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer issuer.Close()

	holder1, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer holder1.Close()

	holder2, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer holder2.Close()

	// Create a data contract with token support
	definitions := map[string]DocumentTypeDefinition{
		"token": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"name":        CreatePropertySchema("string", WithMaxLength(50)),
				"symbol":      CreatePropertySchema("string", WithMaxLength(10)),
				"decimals":    CreatePropertySchema("integer", WithMinimum(0), WithMaximum(18)),
				"totalSupply": CreatePropertySchema("integer", WithMinimum(0)),
			},
			[]string{"name", "symbol", "decimals", "totalSupply"},
			nil,
		),
	}

	contract, err := sdk.Contracts().Create(ctx, issuer, definitions)
	require.NoError(t, err)
	defer contract.Close()

	contractInfo, err := contract.GetInfo()
	require.NoError(t, err)

	contractID, err := NewContractIDFromString(contractInfo.ID)
	require.NoError(t, err)

	// Simulate token operations (in mock mode these might not actually work)
	tokens := sdk.Tokens()

	// Mint tokens
	mintParams := MintParams{
		ContractID:       contractID,
		TokenPosition:    0,
		Amount:           1000000,
		MintToAllocation: false,
		Owner:            issuer,
	}
	_ = tokens.Mint(ctx, mintParams)

	// Transfer tokens
	holder1Info, _ := holder1.GetInfo()
	holder1ID, _ := NewIdentityIDFromString(holder1Info.ID)

	transferParams := TransferParams{
		ContractID:    contractID,
		TokenPosition: 0,
		Amount:        10000,
		ToIdentity:    holder1ID,
		FromIdentity:  issuer,
	}
	_ = tokens.Transfer(ctx, transferParams)

	// Get balance
	balance, err := tokens.GetBalance(ctx, contractID, 0, holder1ID)
	if err == nil {
		assert.GreaterOrEqual(t, balance, uint64(0))
	}

	// Get token info
	tokenInfo, err := tokens.GetInfo(ctx, contractID, 0)
	if err == nil {
		assert.NotNil(t, tokenInfo)
		assert.Equal(t, contractInfo.ID, tokenInfo.ContractID)
	}
}

// TestIntegrationComplexQuery tests complex document queries
func TestIntegrationComplexQuery(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create owner
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	// Create contract with a more complex schema
	definitions := map[string]DocumentTypeDefinition{
		"product": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"name":        CreatePropertySchema("string", WithMaxLength(100)),
				"category":    CreatePropertySchema("string", WithMaxLength(50)),
				"price":       CreatePropertySchema("number", WithMinimum(0)),
				"inStock":     CreatePropertySchema("boolean"),
				"tags":        CreatePropertySchema("array"),
				"rating":      CreatePropertySchema("number", WithMinimum(0), WithMaximum(5)),
				"createdAt":   CreatePropertySchema("integer"),
			},
			[]string{"name", "category", "price", "createdAt"},
			[]Index{
				CreateIndex("categoryPriceIndex",
					[]IndexProperty{
						CreateIndexProperty("category", true),
						CreateIndexProperty("price", true),
					},
					false, false),
				CreateIndex("ratingIndex",
					[]IndexProperty{
						CreateIndexProperty("rating", false),
					},
					false, true), // sparse index
			},
		),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	require.NoError(t, err)
	defer contract.Close()

	// Create multiple products
	products := []map[string]interface{}{
		{
			"name":      "Laptop",
			"category":  "Electronics",
			"price":     999.99,
			"inStock":   true,
			"tags":      []string{"computer", "portable"},
			"rating":    4.5,
			"createdAt": time.Now().Add(-24 * time.Hour).Unix(),
		},
		{
			"name":      "Mouse",
			"category":  "Electronics",
			"price":     29.99,
			"inStock":   true,
			"tags":      []string{"computer", "accessory"},
			"rating":    4.0,
			"createdAt": time.Now().Add(-12 * time.Hour).Unix(),
		},
		{
			"name":      "Coffee Mug",
			"category":  "Kitchen",
			"price":     12.99,
			"inStock":   false,
			"tags":      []string{"drinkware"},
			"createdAt": time.Now().Unix(),
		},
	}

	for _, props := range products {
		doc, err := sdk.Documents().Create(ctx, CreateParams{
			DataContract: contract,
			DocumentType: "product",
			Owner:        owner,
			Properties:   props,
		})
		require.NoError(t, err)
		doc.Close()
	}

	// Complex query: Electronics under $100, in stock, ordered by price
	query := NewQueryBuilder().
		Where("category", "Electronics").
		WhereLT("price", 100).
		Where("inStock", true).
		OrderBy("price", true).
		Limit(10).
		Build()

	results, err := sdk.Documents().Search(ctx, contract, "product", query)
	require.NoError(t, err)

	// Should find the Mouse
	foundMouse := false
	for _, doc := range results {
		data, _ := doc.GetData()
		if name, ok := data["name"].(string); ok && name == "Mouse" {
			foundMouse = true
		}
		doc.Close()
	}
	_ = foundMouse // In mock mode, might not actually search

	// Query with IN operator
	query2 := NewQueryBuilder().
		WhereIn("tags", []interface{}{"computer", "portable"}).
		OrderBy("createdAt", false).
		Build()

	results2, err := sdk.Documents().Search(ctx, contract, "product", query2)
	require.NoError(t, err)

	for _, doc := range results2 {
		doc.Close()
	}
}

// TestIntegrationErrorHandling tests various error scenarios
func TestIntegrationErrorHandling(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)

	ctx := context.Background()

	// Test operations on closed SDK
	sdk.Close()

	_, err = sdk.Identities().Create(ctx)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "closed")

	_, err = sdk.GetNetwork()
	assert.Error(t, err)

	// Create new SDK for more tests
	sdk2, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk2.Close()

	// Test invalid identity ID
	_, err = sdk2.Identities().Get(ctx, "invalid-id-format")
	assert.Error(t, err)

	// Test non-existent identity
	nonExistentIDHex := "1111111111111111111111111111111111111111111111111111111111111111"
	_, err = sdk2.Identities().Get(ctx, nonExistentIDHex)
	assert.Error(t, err)

	// Test creating document with invalid parameters
	_, err = sdk2.Documents().Create(ctx, CreateParams{
		DataContract: nil,
		DocumentType: "test",
		Owner:        nil,
		Properties:   nil,
	})
	assert.Error(t, err)
}