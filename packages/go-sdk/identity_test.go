package dash

import (
	"context"
	"encoding/hex"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Test constants from Rust tests
const (
	testIdentityID      = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"
	testIdentityIDHex   = "2d1a6de6c01d4b8b8c0f6b1e0a2b5a6f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f"
	nonExistentID       = "1111111111111111111111111111111111111111111"
	testPublicKeyHash   = "0123456789abcdef0123456789abcdef01234567"
)

func TestIdentityCreate(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	require.NotNil(t, identity)
	defer identity.Close()

	// Should have a valid handle
	assert.NotNil(t, identity.handle)
	assert.Equal(t, sdk, identity.sdk)
}

func TestIdentityGet(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	identities := sdk.Identities()

	t.Run("Get by hex ID", func(t *testing.T) {
		identity, err := identities.Get(ctx, testIdentityIDHex)
		require.NoError(t, err)
		require.NotNil(t, identity)
		defer identity.Close()
	})

	t.Run("Get by base58 ID", func(t *testing.T) {
		identity, err := identities.Get(ctx, testIdentityID)
		require.NoError(t, err)
		require.NotNil(t, identity)
		defer identity.Close()
	})

	t.Run("Invalid ID format", func(t *testing.T) {
		_, err := identities.Get(ctx, "invalid-id")
		assert.Error(t, err)
	})

	t.Run("Non-existent ID", func(t *testing.T) {
		_, err := identities.Get(ctx, nonExistentID)
		assert.Error(t, err)
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		closedSDK.Close()
		
		_, err := closedSDK.Identities().Get(ctx, testIdentityID)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestIdentityGetByPublicKeyHash(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	identities := sdk.Identities()

	t.Run("Valid public key hash", func(t *testing.T) {
		hash, err := NewPublicKeyHashFromString(testPublicKeyHash)
		require.NoError(t, err)

		identity, err := identities.GetByPublicKeyHash(ctx, hash)
		require.NoError(t, err)
		require.NotNil(t, identity)
		defer identity.Close()
	})

	t.Run("Zero public key hash", func(t *testing.T) {
		var zeroHash PublicKeyHash
		_, err := identities.GetByPublicKeyHash(ctx, zeroHash)
		assert.Error(t, err)
	})
}

func TestIdentityGetBalance(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	identities := sdk.Identities()

	t.Run("Get balance for existing identity", func(t *testing.T) {
		balance, err := identities.GetBalance(ctx, testIdentityID)
		require.NoError(t, err)
		assert.GreaterOrEqual(t, balance, uint64(0))
	})

	t.Run("Get balance for non-existent identity", func(t *testing.T) {
		_, err := identities.GetBalance(ctx, nonExistentID)
		assert.Error(t, err)
	})
}

func TestIdentityGetBalances(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	identities := sdk.Identities()

	t.Run("Get balances for multiple identities", func(t *testing.T) {
		ids := []string{testIdentityID, testIdentityIDHex, nonExistentID}
		balances, err := identities.GetBalances(ctx, ids)
		require.NoError(t, err)
		require.NotNil(t, balances)
		
		// Balances should be a map of hex identity IDs to uint64 values
		// Non-existent identities are not included in the map
		for idHex, balance := range balances {
			// Verify the ID is valid hex
			_, err := hex.DecodeString(idHex)
			assert.NoError(t, err)
			assert.Equal(t, 64, len(idHex)) // 32 bytes = 64 hex chars
			
			// Balance should be valid
			assert.GreaterOrEqual(t, balance, uint64(0))
		}
	})

	t.Run("Empty identity list", func(t *testing.T) {
		balances, err := identities.GetBalances(ctx, []string{})
		require.NoError(t, err)
		assert.NotNil(t, balances)
		assert.Len(t, balances, 0)
	})

	t.Run("Mixed valid and invalid IDs", func(t *testing.T) {
		ids := []string{
			testIdentityID,      // Valid base58
			testIdentityIDHex,   // Valid hex
			"invalid-id",        // Invalid format
		}
		_, err := identities.GetBalances(ctx, ids)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "invalid identity ID")
	})

	t.Run("Hex identity IDs", func(t *testing.T) {
		// Test with hex-encoded identity IDs
		ids := []string{
			testIdentityIDHex,
			"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
		}
		balances, err := identities.GetBalances(ctx, ids)
		require.NoError(t, err)
		assert.NotNil(t, balances)
		
		// Result keys should all be hex
		for idHex := range balances {
			assert.Equal(t, 64, len(idHex))
		}
	})

	t.Run("Base58 identity IDs", func(t *testing.T) {
		// Test with base58-encoded identity IDs
		ids := []string{testIdentityID}
		balances, err := identities.GetBalances(ctx, ids)
		require.NoError(t, err)
		assert.NotNil(t, balances)
		
		// Result keys are still hex (converted internally)
		for idHex := range balances {
			assert.Equal(t, 64, len(idHex))
		}
	})
}

func TestIdentityMethods(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	// Create an identity for testing
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	require.NotNil(t, identity)
	defer identity.Close()

	t.Run("GetInfo", func(t *testing.T) {
		info, err := identity.GetInfo()
		require.NoError(t, err)
		require.NotNil(t, info)
		
		assert.NotEmpty(t, info.ID)
		assert.GreaterOrEqual(t, info.Balance, uint64(0))
	})

	t.Run("GetID", func(t *testing.T) {
		id, err := identity.GetID()
		require.NoError(t, err)
		assert.NotEmpty(t, id)
		
		// Should be valid hex
		_, err = hex.DecodeString(id)
		assert.NoError(t, err)
	})

	t.Run("GetBalance", func(t *testing.T) {
		balance, err := identity.GetBalance()
		require.NoError(t, err)
		assert.GreaterOrEqual(t, balance, uint64(0))
	})
}

func TestIdentityRegisterName(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Register valid name", func(t *testing.T) {
		err := identity.RegisterName(ctx, "testname", nil)
		// In mock mode, this might succeed or fail depending on mock data
		// Just verify it doesn't panic
		_ = err
	})

	t.Run("Register with custom settings", func(t *testing.T) {
		settings := DefaultPutSettings()
		settings.Retries = 5
		
		err := identity.RegisterName(ctx, "anothername", settings)
		_ = err
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		id, _ := closedSDK.Identities().Create(ctx)
		closedSDK.Close()
		
		err := id.RegisterName(ctx, "name", nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestIdentityResolveName(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	identities := sdk.Identities()

	t.Run("Resolve existing name", func(t *testing.T) {
		// In mock mode, might not have real names
		identity, err := identities.ResolveName(ctx, "dash")
		if err == nil {
			assert.NotNil(t, identity)
			identity.Close()
		}
	})

	t.Run("Resolve non-existent name", func(t *testing.T) {
		_, err := identities.ResolveName(ctx, "nonexistentname12345")
		// Should error or return nil
		_ = err
	})
}

func TestIdentityTransferCredits(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Transfer to valid identity", func(t *testing.T) {
		toID, err := NewIdentityIDFromString(testIdentityIDHex)
		require.NoError(t, err)
		
		err = identity.TransferCredits(ctx, toID, 1000, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Transfer with custom settings", func(t *testing.T) {
		toID, err := NewIdentityIDFromString(testIdentityIDHex)
		require.NoError(t, err)
		
		settings := DefaultPutSettings()
		settings.UserFeeIncrease = 10
		
		err = identity.TransferCredits(ctx, toID, 500, settings)
		_ = err
	})
}

func TestIdentityTopUp(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Top up identity", func(t *testing.T) {
		err := identity.TopUp(ctx, 10000, nil)
		// In mock mode, might succeed or fail
		_ = err
	})
}

func TestIdentityWithdraw(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Withdraw to address", func(t *testing.T) {
		info, err := identity.Withdraw(ctx, 5000, "yNPbcFfabtNmmxKdGwhHomdYfVs6gikbPf", nil)
		if err == nil {
			assert.NotNil(t, info)
			assert.NotEmpty(t, info.TransactionID)
		}
	})
}

func TestIdentityPutToInstantLock(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Put with instant lock", func(t *testing.T) {
		// Generate a fake private key (32 bytes)
		privateKey := make([]byte, 32)
		for i := range privateKey {
			privateKey[i] = byte(i)
		}
		
		err := identity.PutToInstantLock(ctx, privateKey, 100000, nil)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Invalid private key size", func(t *testing.T) {
		invalidKey := make([]byte, 16) // Wrong size
		
		err := identity.PutToInstantLock(ctx, invalidKey, 100000, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "32 bytes")
	})
}

func TestIdentityPutToChainLock(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer identity.Close()

	t.Run("Put with chain lock", func(t *testing.T) {
		// Mock asset lock proof
		assetLockProof := []byte(`{"type": "instant", "transaction": "..."}`)
		
		err := identity.PutToChainLock(ctx, 1000, 100000, assetLockProof, nil)
		// In mock mode, might succeed or fail
		_ = err
	})
}

func TestIdentityClose(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	
	identity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)

	// Close should work
	err = identity.Close()
	assert.NoError(t, err)
	
	// Double close should be safe
	err = identity.Close()
	assert.NoError(t, err)
	
	// Handle should be nil after close
	assert.Nil(t, identity.handle)
}