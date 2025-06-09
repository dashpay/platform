package dashsdk

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestDocumentPropertyOperations(t *testing.T) {
	// Skip if we can't create a test SDK
	sdk, err := createTestSDK()
	if err != nil {
		t.Skip("Cannot create test SDK:", err)
	}
	defer sdk.Close()

	// Create test identity
	identity, err := sdk.Identities().Create()
	require.NoError(t, err)
	require.NotNil(t, identity)

	// Create test contract
	contractDef := `{
		"testDoc": {
			"type": "object",
			"properties": {
				"name": {
					"type": "string"
				},
				"age": {
					"type": "integer"
				},
				"address": {
					"type": "object",
					"properties": {
						"street": {
							"type": "string"
						},
						"city": {
							"type": "string"
						}
					}
				},
				"tags": {
					"type": "array",
					"items": {
						"type": "string"
					}
				}
			},
			"required": ["name"],
			"additionalProperties": false
		}
	}`

	contract, err := sdk.Contracts().Create(identity, contractDef)
	require.NoError(t, err)
	require.NotNil(t, contract)

	t.Run("UpdateHandle", func(t *testing.T) {
		// Create document
		doc, err := contract.CreateDocument("testDoc", identity, map[string]interface{}{
			"name": "Alice",
			"age":  25,
		})
		require.NoError(t, err)
		require.NotNil(t, doc)

		// Update properties using Set method
		err = doc.Set("name", "Bob")
		require.NoError(t, err)

		err = doc.Set("age", 30)
		require.NoError(t, err)

		// Verify changes
		info, err := doc.GetInfo()
		require.NoError(t, err)
		assert.Equal(t, "Bob", info.Data["name"])
		assert.Equal(t, float64(30), info.Data["age"]) // JSON numbers are float64
	})

	t.Run("SetProperty", func(t *testing.T) {
		// Create document with nested data
		doc, err := contract.CreateDocument("testDoc", identity, map[string]interface{}{
			"name": "Charlie",
			"address": map[string]interface{}{
				"street": "123 Main St",
				"city":   "Boston",
			},
		})
		require.NoError(t, err)
		require.NotNil(t, doc)

		// Update nested property using path
		err = doc.SetProperty("address.city", "New York")
		require.NoError(t, err)

		// Verify change
		info, err := doc.GetInfo()
		require.NoError(t, err)
		address := info.Data["address"].(map[string]interface{})
		assert.Equal(t, "New York", address["city"])
		assert.Equal(t, "123 Main St", address["street"]) // Other properties unchanged

		// Set array property
		err = doc.SetProperty("tags", []string{"developer", "golang"})
		require.NoError(t, err)

		// Verify array
		info, err = doc.GetInfo()
		require.NoError(t, err)
		tags := info.Data["tags"].([]interface{})
		assert.Len(t, tags, 2)
		assert.Equal(t, "developer", tags[0])
		assert.Equal(t, "golang", tags[1])
	})

	t.Run("RemoveProperty", func(t *testing.T) {
		// Create document with optional properties
		doc, err := contract.CreateDocument("testDoc", identity, map[string]interface{}{
			"name": "David",
			"age":  40,
			"address": map[string]interface{}{
				"street": "456 Oak Ave",
				"city":   "Seattle",
			},
		})
		require.NoError(t, err)
		require.NotNil(t, doc)

		// Remove optional property
		err = doc.RemoveProperty("age")
		require.NoError(t, err)

		// Verify removal
		info, err := doc.GetInfo()
		require.NoError(t, err)
		_, exists := info.Data["age"]
		assert.False(t, exists)
		assert.Equal(t, "David", info.Data["name"]) // Required property still there

		// Remove nested property
		err = doc.RemoveProperty("address.street")
		require.NoError(t, err)

		// Verify nested removal
		info, err = doc.GetInfo()
		require.NoError(t, err)
		address := info.Data["address"].(map[string]interface{})
		_, exists = address["street"]
		assert.False(t, exists)
		assert.Equal(t, "Seattle", address["city"]) // Other nested property still there
	})

	t.Run("ReadOnlyDocument", func(t *testing.T) {
		// Create a read-only document (simulating search results)
		readOnlyDoc := &Document{
			handle:       nil, // No handle
			sdk:          sdk,
			documentType: "testDoc",
			dataContract: contract,
		}

		// All write operations should fail
		err := readOnlyDoc.Set("name", "Eve")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "cannot update document without handle")

		err = readOnlyDoc.SetProperty("name", "Eve")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "cannot set property document without handle")

		err = readOnlyDoc.RemoveProperty("name")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "cannot remove property document without handle")
	})
}