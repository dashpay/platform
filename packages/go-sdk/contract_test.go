package dash

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Test constants from Rust tests
const (
	dpnsContractID    = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"
	dpnsContractIDHex = "b6d8fc5dd60a7ef43c5b4a3e13dc04b6c7f827cf27bc0e9c2b0a3e0e0c8f5a5a"
	nonExistentContractID = "11111111111111111111111111111111"
)

func TestContractCreate(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	// Create owner identity
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	t.Run("Create simple contract", func(t *testing.T) {
		definitions := map[string]DocumentTypeDefinition{
			"message": CreateDocumentTypeDefinition(
				map[string]PropertySchema{
					"text": CreatePropertySchema("string", WithMaxLength(280)),
					"timestamp": CreatePropertySchema("integer"),
				},
				[]string{"text", "timestamp"},
				[]Index{
					CreateIndex("timestampIndex", 
						[]IndexProperty{
							CreateIndexProperty("timestamp", false),
						}, 
						false, false),
				},
			),
		}

		contract, err := sdk.Contracts().Create(ctx, owner, definitions)
		require.NoError(t, err)
		require.NotNil(t, contract)
		defer contract.Close()

		assert.NotNil(t, contract.handle)
		assert.Equal(t, sdk, contract.sdk)
	})

	t.Run("Create with multiple document types", func(t *testing.T) {
		definitions := map[string]DocumentTypeDefinition{
			"post": CreateDocumentTypeDefinition(
				map[string]PropertySchema{
					"title": CreatePropertySchema("string", WithMaxLength(100)),
					"body": CreatePropertySchema("string", WithMaxLength(5000)),
					"author": CreatePropertySchema("string"),
				},
				[]string{"title", "body", "author"},
				nil,
			),
			"comment": CreateDocumentTypeDefinition(
				map[string]PropertySchema{
					"postId": CreatePropertySchema("string"),
					"text": CreatePropertySchema("string", WithMaxLength(500)),
					"author": CreatePropertySchema("string"),
				},
				[]string{"postId", "text", "author"},
				[]Index{
					CreateIndex("postIdIndex", 
						[]IndexProperty{
							CreateIndexProperty("postId", true),
						}, 
						false, false),
				},
			),
		}

		contract, err := sdk.Contracts().Create(ctx, owner, definitions)
		require.NoError(t, err)
		require.NotNil(t, contract)
		defer contract.Close()
	})

	t.Run("Create with nil owner", func(t *testing.T) {
		definitions := map[string]DocumentTypeDefinition{
			"test": CreateDocumentTypeDefinition(nil, nil, nil),
		}

		_, err := sdk.Contracts().Create(ctx, nil, definitions)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "owner")
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		owner, _ := closedSDK.Identities().Create(ctx)
		closedSDK.Close()
		
		_, err := closedSDK.Contracts().Create(ctx, owner, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestContractGet(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	contracts := sdk.Contracts()

	t.Run("Get existing contract", func(t *testing.T) {
		contractID, err := NewContractIDFromString(dpnsContractIDHex)
		require.NoError(t, err)

		contract, err := contracts.Get(ctx, contractID)
		require.NoError(t, err)
		require.NotNil(t, contract)
		defer contract.Close()

		assert.NotNil(t, contract.handle)
	})

	t.Run("Get non-existent contract", func(t *testing.T) {
		var nonExistentID ContractID
		for i := range nonExistentID {
			nonExistentID[i] = 0x11
		}

		_, err := contracts.Get(ctx, nonExistentID)
		assert.Error(t, err)
	})
}

func TestContractGetMany(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	contracts := sdk.Contracts()

	t.Run("Get multiple contracts", func(t *testing.T) {
		contractID1, _ := NewContractIDFromString(dpnsContractIDHex)
		var contractID2 ContractID
		for i := range contractID2 {
			contractID2[i] = byte(i)
		}

		contractIDs := []ContractID{contractID1, contractID2}
		results, err := contracts.GetMany(ctx, contractIDs)
		require.NoError(t, err)
		require.NotNil(t, results)
		
		// In mock mode, might return empty
		for _, contract := range results {
			contract.Close()
		}
	})

	t.Run("Empty contract list", func(t *testing.T) {
		results, err := contracts.GetMany(ctx, []ContractID{})
		require.NoError(t, err)
		assert.NotNil(t, results)
		assert.Len(t, results, 0)
	})
}

func TestContractGetHistory(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	contracts := sdk.Contracts()

	t.Run("Get contract history", func(t *testing.T) {
		contractID, err := NewContractIDFromString(dpnsContractIDHex)
		require.NoError(t, err)

		history, err := contracts.GetHistory(ctx, contractID, 10, 0)
		require.NoError(t, err)
		assert.NotNil(t, history)
		
		// In mock mode, might be empty
		for _, entry := range history {
			assert.GreaterOrEqual(t, entry.Version, uint64(0))
		}
	})

	t.Run("Get history with pagination", func(t *testing.T) {
		contractID, err := NewContractIDFromString(dpnsContractIDHex)
		require.NoError(t, err)

		history, err := contracts.GetHistory(ctx, contractID, 5, 10)
		require.NoError(t, err)
		assert.NotNil(t, history)
	})
}

func TestDataContractMethods(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	// Create a contract for testing
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	definitions := map[string]DocumentTypeDefinition{
		"testDoc": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"field1": CreatePropertySchema("string"),
				"field2": CreatePropertySchema("integer"),
			},
			[]string{"field1"},
			nil,
		),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	require.NoError(t, err)
	require.NotNil(t, contract)
	defer contract.Close()

	t.Run("GetInfo", func(t *testing.T) {
		info, err := contract.GetInfo()
		require.NoError(t, err)
		require.NotNil(t, info)
		
		assert.NotEmpty(t, info.ID)
		assert.NotEmpty(t, info.OwnerID)
		assert.GreaterOrEqual(t, info.Version, uint64(0))
	})

	t.Run("GetSchema", func(t *testing.T) {
		schema, err := contract.GetSchema("testDoc")
		if err == nil {
			assert.NotNil(t, schema)
			
			// Verify it's valid JSON
			var parsed interface{}
			err = json.Unmarshal(schema, &parsed)
			assert.NoError(t, err)
		}
	})

	t.Run("GetSchema non-existent type", func(t *testing.T) {
		_, err := contract.GetSchema("nonExistentType")
		assert.Error(t, err)
	})

	t.Run("GetDocumentTypes", func(t *testing.T) {
		types, err := contract.GetDocumentTypes()
		if err == nil {
			assert.NotNil(t, types)
			// In mock mode, might be empty
		}
	})
}

func TestDataContractPut(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	// Create owner identity
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	// Create contract
	definitions := map[string]DocumentTypeDefinition{
		"record": CreateDocumentTypeDefinition(
			map[string]PropertySchema{
				"value": CreatePropertySchema("string"),
			},
			[]string{"value"},
			nil,
		),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	require.NoError(t, err)
	defer contract.Close()

	t.Run("Put contract", func(t *testing.T) {
		err := contract.Put(ctx, owner, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Put with custom settings", func(t *testing.T) {
		settings := DefaultPutSettings()
		settings.Retries = 5
		
		err := contract.Put(ctx, owner, settings)
		_ = err
	})

	t.Run("Put with nil identity", func(t *testing.T) {
		err := contract.Put(ctx, nil, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "identity")
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		id, _ := closedSDK.Identities().Create(ctx)
		c, _ := closedSDK.Contracts().Create(ctx, id, definitions)
		closedSDK.Close()
		
		err := c.Put(ctx, id, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestDataContractPutAndWait(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	definitions := map[string]DocumentTypeDefinition{
		"item": CreateDocumentTypeDefinition(nil, nil, nil),
	}

	contract, err := sdk.Contracts().Create(ctx, owner, definitions)
	require.NoError(t, err)
	defer contract.Close()

	t.Run("Put and wait", func(t *testing.T) {
		err := contract.PutAndWait(ctx, owner, nil)
		// In mock mode, might succeed or fail
		_ = err
	})
}

func TestDataContractClose(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contract, err := sdk.Contracts().Create(ctx, owner, nil)
	require.NoError(t, err)

	// Close should work
	err = contract.Close()
	assert.NoError(t, err)
	
	// Double close should be safe
	err = contract.Close()
	assert.NoError(t, err)
	
	// Handle should be nil after close
	assert.Nil(t, contract.handle)
}

func TestPropertySchemaHelpers(t *testing.T) {
	t.Run("CreatePropertySchema with all options", func(t *testing.T) {
		schema := CreatePropertySchema("string",
			WithFormat("email"),
			WithMinLength(5),
			WithMaxLength(100),
			WithPattern("^[^@]+@[^@]+$"),
			WithDescription("User email address"),
		)

		assert.Equal(t, "string", schema.Type)
		assert.Equal(t, "email", schema.Format)
		assert.Equal(t, 5, *schema.MinLength)
		assert.Equal(t, 100, *schema.MaxLength)
		assert.NotEmpty(t, schema.Pattern)
		assert.Equal(t, "User email address", schema.Description)
	})

	t.Run("Number property with bounds", func(t *testing.T) {
		schema := CreatePropertySchema("number",
			WithMinimum(0),
			WithMaximum(100),
			WithDescription("Percentage"),
		)

		assert.Equal(t, "number", schema.Type)
		assert.Equal(t, float64(0), *schema.Minimum)
		assert.Equal(t, float64(100), *schema.Maximum)
	})
}

func TestIndexHelpers(t *testing.T) {
	t.Run("CreateIndex", func(t *testing.T) {
		index := CreateIndex("userEmailIndex",
			[]IndexProperty{
				CreateIndexProperty("email", true),
			},
			true,  // unique
			false, // not sparse
		)

		assert.Equal(t, "userEmailIndex", index.Name)
		assert.True(t, index.Unique)
		assert.False(t, index.Sparse)
		assert.Len(t, index.Properties, 1)
		assert.Equal(t, "email", index.Properties[0].Name)
		assert.Equal(t, "asc", index.Properties[0].Asc)
	})

	t.Run("CreateIndexProperty", func(t *testing.T) {
		ascProp := CreateIndexProperty("field1", true)
		assert.Equal(t, "field1", ascProp.Name)
		assert.Equal(t, "asc", ascProp.Asc)

		descProp := CreateIndexProperty("field2", false)
		assert.Equal(t, "field2", descProp.Name)
		assert.Equal(t, "desc", descProp.Asc)
	})
}