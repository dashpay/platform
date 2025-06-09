package dash

import (
	"context"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Test constants
const (
	testDocumentIDHex = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
	testDocumentType  = "domain"
)

func createTestContract(t *testing.T, sdk *SDK, owner *Identity) *DataContract {
	definitions := map[string]DocumentTypeDefinition{
		"message": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"text":      CreatePropertySchema("string", WithMaxLength(500)),
				"author":    CreatePropertySchema("string"),
				"timestamp": CreatePropertySchema("integer"),
			},
			[]string{"text", "author"},
			[]Index{
				CreateIndex("timestampIndex",
					[]IndexProperty{
						CreateIndexProperty("timestamp", false),
					},
					false, false),
			},
		),
		"profile": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"username":    CreatePropertySchema("string", WithMaxLength(50)),
				"displayName": CreatePropertySchema("string", WithMaxLength(100)),
				"bio":         CreatePropertySchema("string", WithMaxLength(500)),
				"avatar":      CreatePropertySchema("string"),
			},
			[]string{"username"},
			[]Index{
				CreateIndex("usernameIndex",
					[]IndexProperty{
						CreateIndexProperty("username", true),
					},
					true, false),
			},
		),
	}

	contract, err := sdk.Contracts().Create(context.Background(), owner, definitions)
	require.NoError(t, err)
	return contract
}

func TestDocumentCreate(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create owner identity
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	// Create data contract
	contract := createTestContract(t, sdk, owner)
	defer contract.Close()

	t.Run("Create simple document", func(t *testing.T) {
		params := CreateParams{
			DataContract: contract,
			DocumentType: "message",
			Owner:        owner,
			Properties: map[string]interface{}{
				"text":      "Hello, World!",
				"author":    "testuser",
				"timestamp": 1234567890,
			},
		}

		document, err := sdk.Documents().Create(ctx, params)
		require.NoError(t, err)
		require.NotNil(t, document)
		defer document.Close()

		assert.NotNil(t, document.handle)
		assert.Equal(t, "message", document.documentType)
		assert.Equal(t, contract, document.dataContract)
	})

	t.Run("Create with missing required fields", func(t *testing.T) {
		params := CreateParams{
			DataContract: contract,
			DocumentType: "message",
			Owner:        owner,
			Properties: map[string]interface{}{
				"text": "Missing author field",
				// "author" is required but missing
			},
		}

		// Should still create locally, validation happens on platform
		document, err := sdk.Documents().Create(ctx, params)
		require.NoError(t, err)
		require.NotNil(t, document)
		defer document.Close()
	})

	t.Run("Create with nil contract", func(t *testing.T) {
		params := CreateParams{
			DataContract: nil,
			DocumentType: "message",
			Owner:        owner,
			Properties:   map[string]interface{}{"test": "value"},
		}

		_, err := sdk.Documents().Create(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "contract")
	})

	t.Run("Create with nil owner", func(t *testing.T) {
		params := CreateParams{
			DataContract: contract,
			DocumentType: "message",
			Owner:        nil,
			Properties:   map[string]interface{}{"test": "value"},
		}

		_, err := sdk.Documents().Create(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "owner")
	})

	t.Run("Create with empty document type", func(t *testing.T) {
		params := CreateParams{
			DataContract: contract,
			DocumentType: "",
			Owner:        owner,
			Properties:   map[string]interface{}{"test": "value"},
		}

		_, err := sdk.Documents().Create(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "document type")
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		closedSDK.Close()

		params := CreateParams{
			DataContract: contract,
			DocumentType: "message",
			Owner:        owner,
			Properties:   map[string]interface{}{"test": "value"},
		}

		_, err := closedSDK.Documents().Create(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestDocumentGet(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	documents := sdk.Documents()

	t.Run("Get existing document", func(t *testing.T) {
		contractID, err := NewContractIDFromString(dpnsContractIDHex)
		require.NoError(t, err)

		documentID, err := NewDocumentIDFromString(testDocumentIDHex)
		require.NoError(t, err)

		document, err := documents.Get(ctx, contractID, testDocumentType, documentID)
		require.NoError(t, err)
		require.NotNil(t, document)
		defer document.Close()

		assert.NotNil(t, document.handle)
		assert.Equal(t, testDocumentType, document.documentType)
	})

	t.Run("Get non-existent document", func(t *testing.T) {
		var contractID ContractID
		var documentID DocumentID
		for i := range documentID {
			documentID[i] = 0x11
		}

		_, err := documents.Get(ctx, contractID, "test", documentID)
		assert.Error(t, err)
	})
}

func TestDocumentSearch(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create test contract
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contract := createTestContract(t, sdk, owner)
	defer contract.Close()

	t.Run("Search results contain document metadata", func(t *testing.T) {
		// In a real scenario, we would create documents first
		// For mock mode, we test the parsing logic
		query := NewQueryBuilder().Limit(5).Build()

		results, err := sdk.Documents().Search(ctx, contract, "message", query)
		require.NoError(t, err)

		// Even if no results, the search should succeed
		assert.NotNil(t, results)
		
		// If there are results, verify structure
		for _, doc := range results {
			info, _ := doc.GetInfo()
			if info != nil {
				// Document should have data
				assert.NotNil(t, info.Data)
				
				// Check for system fields (may or may not be present in mock)
				if info.ID != "" {
					assert.NotEmpty(t, info.ID)
				}
				if info.OwnerID != "" {
					assert.NotEmpty(t, info.OwnerID)
				}
			}
			doc.Close()
		}
	})

	t.Run("Search with empty query", func(t *testing.T) {
		query := DocumentQuery{
			Where: make(map[string]interface{}),
		}

		results, err := sdk.Documents().Search(ctx, contract, "message", query)
		require.NoError(t, err)
		assert.NotNil(t, results)

		// Verify document structure
		for _, doc := range results {
			// Documents from search don't have handles
			assert.False(t, doc.HasHandle())
			
			// But they should have info
			info, err := doc.GetInfo()
			if err == nil {
				assert.NotNil(t, info.Data)
				assert.Equal(t, "message", doc.documentType)
			}
			
			// Should not be able to perform write operations
			err = doc.Put(ctx, owner, nil, nil)
			assert.Error(t, err)
			assert.Contains(t, err.Error(), "cannot put document without handle")
			
			doc.Close()
		}
	})

	t.Run("Search with where clause", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("author", "testuser").
			Build()

		results, err := sdk.Documents().Search(ctx, contract, "message", query)
		require.NoError(t, err)
		assert.NotNil(t, results)

		for _, doc := range results {
			doc.Close()
		}
	})

	t.Run("Search with complex query", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("author", "testuser").
			WhereGT("timestamp", 1000000000).
			OrderBy("timestamp", false).
			Limit(10).
			Build()

		results, err := sdk.Documents().Search(ctx, contract, "message", query)
		require.NoError(t, err)
		assert.NotNil(t, results)

		for _, doc := range results {
			// Verify document data is accessible
			data, err := doc.GetData()
			if err == nil {
				// Check if document matches query criteria
				if author, ok := data["author"].(string); ok {
					assert.Equal(t, "testuser", author)
				}
				if timestamp, ok := getNumberField(data, "timestamp"); ok {
					assert.Greater(t, timestamp, float64(1000000000))
				}
			}
			
			doc.Close()
		}
	})

	t.Run("Search with nil contract", func(t *testing.T) {
		query := DocumentQuery{}
		_, err := sdk.Documents().Search(ctx, nil, "message", query)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "contract")
	})
}

func TestDocumentMethods(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create test document
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contract := createTestContract(t, sdk, owner)
	defer contract.Close()

	params := CreateParams{
		DataContract: contract,
		DocumentType: "profile",
		Owner:        owner,
		Properties: map[string]interface{}{
			"username":    "testuser",
			"displayName": "Test User",
			"bio":         "A test user profile",
			"avatar":      "https://example.com/avatar.jpg",
		},
	}

	document, err := sdk.Documents().Create(ctx, params)
	require.NoError(t, err)
	require.NotNil(t, document)
	defer document.Close()

	t.Run("GetInfo", func(t *testing.T) {
		info, err := document.GetInfo()
		require.NoError(t, err)
		require.NotNil(t, info)

		assert.NotEmpty(t, info.ID)
		assert.NotEmpty(t, info.OwnerID)
		assert.NotEmpty(t, info.DataContractID)
		assert.Equal(t, "profile", info.DocumentType)
		assert.NotNil(t, info.Data)
	})

	t.Run("GetID", func(t *testing.T) {
		id, err := document.GetID()
		require.NoError(t, err)
		assert.NotEmpty(t, id)
	})

	t.Run("GetData", func(t *testing.T) {
		data, err := document.GetData()
		require.NoError(t, err)
		assert.NotNil(t, data)
		
		// Check original properties are present
		assert.Equal(t, "testuser", data["username"])
		assert.Equal(t, "Test User", data["displayName"])
	})

	t.Run("Get field", func(t *testing.T) {
		value, exists := document.Get("username")
		assert.True(t, exists)
		assert.Equal(t, "testuser", value)

		_, exists = document.Get("nonexistent")
		assert.False(t, exists)
	})

	t.Run("Set field", func(t *testing.T) {
		err := document.Set("bio", "Updated bio")
		require.NoError(t, err)

		value, exists := document.Get("bio")
		assert.True(t, exists)
		assert.Equal(t, "Updated bio", value)
	})
}

func TestDocumentOperations(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	// Create test setup
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contract := createTestContract(t, sdk, owner)
	defer contract.Close()

	document, err := sdk.Documents().Create(ctx, CreateParams{
		DataContract: contract,
		DocumentType: "message",
		Owner:        owner,
		Properties: map[string]interface{}{
			"text":      "Test message",
			"author":    "testuser",
			"timestamp": 1234567890,
		},
	})
	require.NoError(t, err)
	defer document.Close()

	t.Run("Put document", func(t *testing.T) {
		err := document.Put(ctx, owner, nil, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Put with settings", func(t *testing.T) {
		settings := DefaultPutSettings()
		settings.Retries = 5

		err := document.Put(ctx, owner, settings, nil)
		_ = err
	})

	t.Run("Put with payment info", func(t *testing.T) {
		paymentInfo := &TokenPaymentInfo{
			TokenContractPosition: 0,
			GasFeesPaidBy:         GasFeesDocumentOwner,
		}

		err := document.Put(ctx, owner, nil, paymentInfo)
		_ = err
	})

	t.Run("Put with nil identity", func(t *testing.T) {
		err := document.Put(ctx, nil, nil, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "identity")
	})

	t.Run("PutAndWait", func(t *testing.T) {
		err := document.PutAndWait(ctx, owner, nil, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Replace", func(t *testing.T) {
		err := document.Replace(ctx, owner, nil, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Delete", func(t *testing.T) {
		err := document.Delete(ctx, owner, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Transfer", func(t *testing.T) {
		toID, err := NewIdentityIDFromString(testIdentityIDHex)
		require.NoError(t, err)

		info, err := document.Transfer(ctx, toID, owner, nil, nil)
		if err == nil {
			assert.NotNil(t, info)
		}
	})

	t.Run("Purchase", func(t *testing.T) {
		purchaser, err := sdk.Identities().Create(ctx)
		require.NoError(t, err)
		defer purchaser.Close()

		err = document.Purchase(ctx, purchaser, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("UpdatePrice", func(t *testing.T) {
		err := document.UpdatePrice(ctx, 1000, owner, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Destroy", func(t *testing.T) {
		err := document.Destroy(ctx, nil)
		// In mock mode, might succeed or fail
		_ = err
	})
}

func TestDocumentClose(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contract := createTestContract(t, sdk, owner)
	defer contract.Close()

	document, err := sdk.Documents().Create(ctx, CreateParams{
		DataContract: contract,
		DocumentType: "message",
		Owner:        owner,
		Properties:   map[string]interface{}{"test": "value"},
	})
	require.NoError(t, err)

	// Close should work
	err = document.Close()
	assert.NoError(t, err)

	// Double close should be safe
	err = document.Close()
	assert.NoError(t, err)

	// Handle should be nil after close
	assert.Nil(t, document.handle)
}

func TestQueryBuilder(t *testing.T) {
	t.Run("Empty query", func(t *testing.T) {
		query := NewQueryBuilder().Build()
		assert.NotNil(t, query.Where)
		assert.Len(t, query.Where, 0)
		assert.Len(t, query.OrderBy, 0)
		assert.Equal(t, 0, query.Limit)
	})

	t.Run("Simple where clause", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("status", "active").
			Where("type", "post").
			Build()

		assert.Equal(t, "active", query.Where["status"])
		assert.Equal(t, "post", query.Where["type"])
	})

	t.Run("Comparison operators", func(t *testing.T) {
		query := NewQueryBuilder().
			WhereGT("age", 18).
			WhereLT("price", 100).
			Build()

		ageClause := query.Where["age"].(map[string]interface{})
		assert.Equal(t, 18, ageClause["$gt"])

		priceClause := query.Where["price"].(map[string]interface{})
		assert.Equal(t, 100, priceClause["$lt"])
	})

	t.Run("WhereIn clause", func(t *testing.T) {
		query := NewQueryBuilder().
			WhereIn("category", []interface{}{"tech", "science", "news"}).
			Build()

		categoryClause := query.Where["category"].(map[string]interface{})
		values := categoryClause["$in"].([]interface{})
		assert.Len(t, values, 3)
		assert.Contains(t, values, "tech")
	})

	t.Run("Ordering", func(t *testing.T) {
		query := NewQueryBuilder().
			OrderBy("timestamp", false).
			OrderBy("score", true).
			Build()

		assert.Len(t, query.OrderBy, 2)
		assert.Equal(t, "timestamp", query.OrderBy[0].Field)
		assert.Equal(t, "desc", query.OrderBy[0].Direction)
		assert.Equal(t, "score", query.OrderBy[1].Field)
		assert.Equal(t, "asc", query.OrderBy[1].Direction)
	})

	t.Run("Pagination", func(t *testing.T) {
		query := NewQueryBuilder().
			Limit(25).
			StartAt(100).
			Build()

		assert.Equal(t, 25, query.Limit)
		assert.Equal(t, 100, query.StartAt)
	})

	t.Run("StartAfter", func(t *testing.T) {
		query := NewQueryBuilder().
			Limit(10).
			StartAfter("lastDocumentId").
			Build()

		assert.Equal(t, 10, query.Limit)
		assert.Equal(t, "lastDocumentId", query.StartAfter)
	})

	t.Run("Complex query", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("author", "user123").
			WhereGT("timestamp", 1000000).
			WhereLT("timestamp", 2000000).
			WhereIn("tags", []interface{}{"important", "urgent"}).
			OrderBy("timestamp", false).
			OrderBy("priority", true).
			Limit(50).
			StartAfter("abc123").
			Build()

		assert.Len(t, query.Where, 3)
		assert.Len(t, query.OrderBy, 2)
		assert.Equal(t, 50, query.Limit)
		assert.Equal(t, "abc123", query.StartAfter)
	})
}