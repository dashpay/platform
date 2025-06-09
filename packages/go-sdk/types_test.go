package dash

import (
	"encoding/hex"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestIdentityID(t *testing.T) {
	t.Run("NewIdentityIDFromString", func(t *testing.T) {
		// Valid hex string
		hexStr := "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
		id, err := NewIdentityIDFromString(hexStr)
		require.NoError(t, err)
		assert.Equal(t, hexStr, id.String())

		// Invalid hex
		_, err = NewIdentityIDFromString("invalid-hex")
		assert.Error(t, err)

		// Wrong length
		_, err = NewIdentityIDFromString("0123456789abcdef")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "32 bytes")
	})

	t.Run("String", func(t *testing.T) {
		var id IdentityID
		for i := range id {
			id[i] = byte(i)
		}
		hexStr := id.String()
		assert.Equal(t, 64, len(hexStr))
		
		// Verify it's valid hex
		decoded, err := hex.DecodeString(hexStr)
		require.NoError(t, err)
		assert.Equal(t, id[:], decoded)
	})
}

func TestContractID(t *testing.T) {
	t.Run("NewContractIDFromString", func(t *testing.T) {
		// Valid hex string
		hexStr := "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
		id, err := NewContractIDFromString(hexStr)
		require.NoError(t, err)
		assert.Equal(t, hexStr, id.String())

		// Invalid hex
		_, err = NewContractIDFromString("zzzzzz")
		assert.Error(t, err)

		// Wrong length
		_, err = NewContractIDFromString("abcd")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "32 bytes")
	})
}

func TestDocumentID(t *testing.T) {
	t.Run("NewDocumentIDFromString", func(t *testing.T) {
		// Valid hex string
		hexStr := "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
		id, err := NewDocumentIDFromString(hexStr)
		require.NoError(t, err)
		assert.Equal(t, hexStr, id.String())

		// Invalid cases
		_, err = NewDocumentIDFromString("not-hex")
		assert.Error(t, err)

		_, err = NewDocumentIDFromString("12345678")
		assert.Error(t, err)
	})
}

func TestPublicKeyHash(t *testing.T) {
	t.Run("NewPublicKeyHashFromString", func(t *testing.T) {
		// Valid hex string (20 bytes = 40 hex chars)
		hexStr := "0123456789abcdef0123456789abcdef01234567"
		hash, err := NewPublicKeyHashFromString(hexStr)
		require.NoError(t, err)
		assert.Equal(t, hexStr, hash.String())

		// Invalid hex
		_, err = NewPublicKeyHashFromString("gggggg")
		assert.Error(t, err)

		// Wrong length
		_, err = NewPublicKeyHashFromString("0123456789abcdef0123456789abcdef0123456789abcdef")
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "20 bytes")
	})

	t.Run("String", func(t *testing.T) {
		var hash PublicKeyHash
		for i := range hash {
			hash[i] = byte(i * 10)
		}
		hexStr := hash.String()
		assert.Equal(t, 40, len(hexStr))
	})
}

func TestGasFeesPaidBy(t *testing.T) {
	tests := []struct {
		value    GasFeesPaidBy
		expected int
	}{
		{GasFeesDocumentOwner, 0},
		{GasFeesContractOwner, 1},
		{GasFeesPreferContractOwner, 2},
	}

	for _, tt := range tests {
		assert.Equal(t, tt.expected, int(tt.value))
	}
}

func TestDocumentTypeDefinition(t *testing.T) {
	props := map[string]PropertySchema{
		"name": CreatePropertySchema("string", WithMaxLength(100)),
		"age":  CreatePropertySchema("integer", WithMinimum(0), WithMaximum(150)),
	}
	required := []string{"name"}
	indices := []Index{
		CreateIndex("nameIndex", []IndexProperty{
			CreateIndexProperty("name", true),
		}, true, false),
	}

	def := CreateDocumentTypeDefinition(props, required, indices)
	
	assert.Equal(t, "object", def.Type)
	assert.Equal(t, props, def.Properties)
	assert.Equal(t, required, def.Required)
	assert.Equal(t, indices, def.Indices)
	assert.False(t, def.AdditionalProperties)
}

func TestPropertySchema(t *testing.T) {
	t.Run("String property with constraints", func(t *testing.T) {
		schema := CreatePropertySchema("string",
			WithFormat("email"),
			WithMinLength(5),
			WithMaxLength(255),
			WithPattern("^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"),
			WithDescription("Email address"),
		)

		assert.Equal(t, "string", schema.Type)
		assert.Equal(t, "email", schema.Format)
		assert.Equal(t, 5, *schema.MinLength)
		assert.Equal(t, 255, *schema.MaxLength)
		assert.NotEmpty(t, schema.Pattern)
		assert.Equal(t, "Email address", schema.Description)
	})

	t.Run("Number property with constraints", func(t *testing.T) {
		schema := CreatePropertySchema("number",
			WithMinimum(0),
			WithMaximum(100),
			WithDescription("Percentage value"),
		)

		assert.Equal(t, "number", schema.Type)
		assert.Equal(t, float64(0), *schema.Minimum)
		assert.Equal(t, float64(100), *schema.Maximum)
		assert.Equal(t, "Percentage value", schema.Description)
	})
}

func TestIndex(t *testing.T) {
	t.Run("Simple index", func(t *testing.T) {
		index := CreateIndex("userIndex",
			[]IndexProperty{
				CreateIndexProperty("username", true),
			},
			true,  // unique
			false, // not sparse
		)

		assert.Equal(t, "userIndex", index.Name)
		assert.Len(t, index.Properties, 1)
		assert.Equal(t, "username", index.Properties[0].Name)
		assert.Equal(t, "asc", index.Properties[0].Asc)
		assert.True(t, index.Unique)
		assert.False(t, index.Sparse)
	})

	t.Run("Compound index", func(t *testing.T) {
		index := CreateIndex("timestampAuthorIndex",
			[]IndexProperty{
				CreateIndexProperty("timestamp", false), // descending
				CreateIndexProperty("author", true),     // ascending
			},
			false, // not unique
			true,  // sparse
		)

		assert.Equal(t, "timestampAuthorIndex", index.Name)
		assert.Len(t, index.Properties, 2)
		assert.Equal(t, "desc", index.Properties[0].Asc)
		assert.Equal(t, "asc", index.Properties[1].Asc)
		assert.False(t, index.Unique)
		assert.True(t, index.Sparse)
	})
}

func TestQueryBuilder(t *testing.T) {
	t.Run("Simple query", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("status", "active").
			OrderBy("createdAt", false).
			Limit(10).
			Build()

		assert.Equal(t, "active", query.Where["status"])
		assert.Len(t, query.OrderBy, 1)
		assert.Equal(t, "createdAt", query.OrderBy[0].Field)
		assert.Equal(t, "desc", query.OrderBy[0].Direction)
		assert.Equal(t, 10, query.Limit)
	})

	t.Run("Complex query", func(t *testing.T) {
		query := NewQueryBuilder().
			Where("type", "post").
			WhereGT("timestamp", 1000).
			WhereLT("score", 100).
			WhereIn("category", []interface{}{"tech", "science"}).
			OrderBy("score", false).
			OrderBy("timestamp", true).
			Limit(20).
			StartAfter("lastDocId").
			Build()

		assert.Equal(t, "post", query.Where["type"])
		assert.Equal(t, map[string]interface{}{"$gt": 1000}, query.Where["timestamp"])
		assert.Equal(t, map[string]interface{}{"$lt": 100}, query.Where["score"])
		
		inClause := query.Where["category"].(map[string]interface{})
		assert.Equal(t, []interface{}{"tech", "science"}, inClause["$in"])
		
		assert.Len(t, query.OrderBy, 2)
		assert.Equal(t, 20, query.Limit)
		assert.Equal(t, "lastDocId", query.StartAfter)
	})
}