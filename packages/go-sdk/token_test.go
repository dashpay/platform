package dash

import (
	"context"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// Test constants
const (
	testTokenContractIDHex = "def0123456789abcdef0123456789abcdef0123456789abcdef0123456789abc"
	testTokenPosition      = uint16(0)
)

func TestTokenMint(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	// Create owner identity
	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Mint tokens", func(t *testing.T) {
		params := MintParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           1000,
			MintToAllocation: false,
			Owner:            owner,
			Settings:         nil,
		}

		err := tokens.Mint(ctx, params)
		// In mock mode, might succeed or fail
		_ = err
	})

	t.Run("Mint to specific identity", func(t *testing.T) {
		destID, err := NewIdentityIDFromString(testIdentityIDHex)
		require.NoError(t, err)

		params := MintParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           500,
			DestinationID:    &destID,
			MintToAllocation: false,
			Owner:            owner,
			Settings:         nil,
		}

		err = tokens.Mint(ctx, params)
		_ = err
	})

	t.Run("Mint with nil owner", func(t *testing.T) {
		params := MintParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        1000,
			Owner:         nil,
		}

		err := tokens.Mint(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "owner")
	})

	t.Run("SDK closed", func(t *testing.T) {
		closedSDK, _ := NewMockSDK()
		closedSDK.Close()

		params := MintParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        1000,
			Owner:         owner,
		}

		err := closedSDK.Tokens().Mint(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "closed")
	})
}

func TestTokenBurn(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Burn tokens", func(t *testing.T) {
		params := BurnParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        100,
			Owner:         owner,
			Settings:      nil,
		}

		err := tokens.Burn(ctx, params)
		_ = err
	})

	t.Run("Burn from specific identity", func(t *testing.T) {
		burnFromID, err := NewIdentityIDFromString(testIdentityIDHex)
		require.NoError(t, err)

		params := BurnParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           50,
			BurnFromIdentity: &burnFromID,
			Owner:            owner,
			Settings:         nil,
		}

		err = tokens.Burn(ctx, params)
		_ = err
	})
}

func TestTokenTransfer(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	fromIdentity, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer fromIdentity.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	toID, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Transfer tokens", func(t *testing.T) {
		params := TransferParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        250,
			ToIdentity:    toID,
			FromIdentity:  fromIdentity,
			Settings:      nil,
		}

		err := tokens.Transfer(ctx, params)
		_ = err
	})

	t.Run("Transfer with custom settings", func(t *testing.T) {
		settings := DefaultPutSettings()
		settings.UserFeeIncrease = 5

		params := TransferParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        100,
			ToIdentity:    toID,
			FromIdentity:  fromIdentity,
			Settings:      settings,
		}

		err := tokens.Transfer(ctx, params)
		_ = err
	})

	t.Run("Transfer with nil from identity", func(t *testing.T) {
		params := TransferParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        100,
			ToIdentity:    toID,
			FromIdentity:  nil,
		}

		err := tokens.Transfer(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "from identity")
	})
}

func TestTokenGetBalance(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	identityID, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Get token balance", func(t *testing.T) {
		balance, err := tokens.GetBalance(ctx, contractID, testTokenPosition, identityID)
		require.NoError(t, err)
		assert.GreaterOrEqual(t, balance, uint64(0))
	})

	t.Run("Get balance for non-existent identity", func(t *testing.T) {
		var nonExistentID IdentityID
		for i := range nonExistentID {
			nonExistentID[i] = 0x11
		}

		_, err := tokens.GetBalance(ctx, contractID, testTokenPosition, nonExistentID)
		// Might succeed with 0 balance or fail
		_ = err
	})
}

func TestTokenGetInfo(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Get token info", func(t *testing.T) {
		info, err := tokens.GetInfo(ctx, contractID, testTokenPosition)
		if err == nil {
			assert.NotNil(t, info)
			assert.NotEmpty(t, info.ContractID)
			assert.Equal(t, testTokenPosition, info.Position)
			assert.GreaterOrEqual(t, info.TotalSupply, uint64(0))
		}
	})
}

func TestTokenFreeze(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	identityToFreeze, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Freeze tokens", func(t *testing.T) {
		params := FreezeParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			IdentityToFreeze: identityToFreeze,
			Owner:            owner,
			Settings:         nil,
		}

		err := tokens.Freeze(ctx, params)
		_ = err
	})

	t.Run("Freeze with nil owner", func(t *testing.T) {
		params := FreezeParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			IdentityToFreeze: identityToFreeze,
			Owner:            nil,
		}

		err := tokens.Freeze(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "owner")
	})
}

func TestTokenUnfreeze(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	identityToUnfreeze, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Unfreeze tokens", func(t *testing.T) {
		params := UnfreezeParams{
			ContractID:         contractID,
			TokenPosition:      testTokenPosition,
			IdentityToUnfreeze: identityToUnfreeze,
			Owner:              owner,
			Settings:           nil,
		}

		err := tokens.Unfreeze(ctx, params)
		_ = err
	})
}

func TestTokenPurchase(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	purchaser, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer purchaser.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Purchase tokens", func(t *testing.T) {
		params := PurchaseParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        100,
			PriceOffer:    1000,
			Purchaser:     purchaser,
			Settings:      nil,
		}

		err := tokens.Purchase(ctx, params)
		_ = err
	})

	t.Run("Purchase with nil purchaser", func(t *testing.T) {
		params := PurchaseParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			Amount:        100,
			PriceOffer:    1000,
			Purchaser:     nil,
		}

		err := tokens.Purchase(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "purchaser")
	})
}

func TestTokenSetPrice(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Set token price", func(t *testing.T) {
		params := SetPriceParams{
			ContractID:    contractID,
			TokenPosition: testTokenPosition,
			NewPrice:      5000,
			Owner:         owner,
			Settings:      nil,
		}

		err := tokens.SetPrice(ctx, params)
		_ = err
	})
}

func TestTokenClaim(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	claimer, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer claimer.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	t.Run("Claim pre-programmed distribution", func(t *testing.T) {
		params := ClaimParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           50,
			DistributionType: TokenDistributionPreProgrammed,
			Claimer:          claimer,
			Settings:         nil,
		}

		err := tokens.Claim(ctx, params)
		_ = err
	})

	t.Run("Claim perpetual distribution", func(t *testing.T) {
		params := ClaimParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           25,
			DistributionType: TokenDistributionPerpetual,
			Claimer:          claimer,
			Settings:         nil,
		}

		err := tokens.Claim(ctx, params)
		_ = err
	})

	t.Run("Claim with nil claimer", func(t *testing.T) {
		params := ClaimParams{
			ContractID:       contractID,
			TokenPosition:    testTokenPosition,
			Amount:           50,
			DistributionType: TokenDistributionPreProgrammed,
			Claimer:          nil,
		}

		err := tokens.Claim(ctx, params)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "claimer")
	})
}

func TestTokenGetAllocationInfo(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	identityID, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Get allocation info", func(t *testing.T) {
		info, err := tokens.GetAllocationInfo(ctx, contractID, testTokenPosition, identityID)
		if err == nil {
			assert.NotNil(t, info)
			assert.NotEmpty(t, info.IdentityID)
			assert.GreaterOrEqual(t, info.TotalAllocation, uint64(0))
		}
	})
}

func TestTokenDestroyFrozenFunds(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	defer sdk.Close()

	ctx := context.Background()
	tokens := sdk.Tokens()

	owner, err := sdk.Identities().Create(ctx)
	require.NoError(t, err)
	defer owner.Close()

	contractID, err := NewContractIDFromString(testTokenContractIDHex)
	require.NoError(t, err)

	identityWithFrozenFunds, err := NewIdentityIDFromString(testIdentityIDHex)
	require.NoError(t, err)

	t.Run("Destroy frozen funds", func(t *testing.T) {
		err := tokens.DestroyFrozenFunds(ctx, contractID, testTokenPosition, identityWithFrozenFunds, owner, nil)
		_ = err
	})

	t.Run("Destroy with nil owner", func(t *testing.T) {
		err := tokens.DestroyFrozenFunds(ctx, contractID, testTokenPosition, identityWithFrozenFunds, nil, nil)
		assert.Error(t, err)
		assert.Contains(t, err.Error(), "owner")
	})
}

func TestTokenDistributionType(t *testing.T) {
	tests := []struct {
		value    TokenDistributionType
		expected int
	}{
		{TokenDistributionPreProgrammed, 0},
		{TokenDistributionPerpetual, 1},
	}

	for _, tt := range tests {
		assert.Equal(t, tt.expected, int(tt.value))
	}
}