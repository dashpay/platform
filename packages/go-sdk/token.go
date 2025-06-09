package dash

// #cgo CFLAGS: -I./internal/ffi
// #include "internal/ffi/dash_sdk_ffi.h"
// #include <stdlib.h>
import "C"
import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"unsafe"

	"github.com/dashpay/platform/packages/go-sdk/internal/ffi"
)

// TokenOperations provides token-related operations
type TokenOperations struct {
	sdk *SDK
}

// Tokens returns the token operations interface
func (s *SDK) Tokens() *TokenOperations {
	return &TokenOperations{sdk: s}
}

// MintParams contains parameters for minting tokens
type MintParams struct {
	ContractID       ContractID
	TokenPosition    uint16
	Amount           uint64
	DestinationID    *IdentityID
	MintToAllocation bool
	Owner            *Identity
	Settings         *PutSettings
}

// Mint mints new tokens
func (t *TokenOperations) Mint(ctx context.Context, params MintParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cSettings := convertPutSettings(params.Settings)

	var cDestID *C.uint8_t
	if params.DestinationID != nil {
		cDestID = (*C.uint8_t)(unsafe.Pointer(&params.DestinationID[0]))
	}

	result := C.dash_sdk_token_mint(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.Amount),
		cDestID,
		C.bool(params.MintToAllocation),
		params.Owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// BurnParams contains parameters for burning tokens
type BurnParams struct {
	ContractID       ContractID
	TokenPosition    uint16
	Amount           uint64
	BurnFromIdentity *IdentityID
	Owner            *Identity
	Settings         *PutSettings
}

// Burn burns tokens
func (t *TokenOperations) Burn(ctx context.Context, params BurnParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cSettings := convertPutSettings(params.Settings)

	var cBurnFromID *C.uint8_t
	if params.BurnFromIdentity != nil {
		cBurnFromID = (*C.uint8_t)(unsafe.Pointer(&params.BurnFromIdentity[0]))
	}

	result := C.dash_sdk_token_burn(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.Amount),
		cBurnFromID,
		params.Owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// TransferParams contains parameters for transferring tokens
type TransferParams struct {
	ContractID    ContractID
	TokenPosition uint16
	Amount        uint64
	ToIdentity    IdentityID
	FromIdentity  *Identity
	Settings      *PutSettings
}

// Transfer transfers tokens between identities
func (t *TokenOperations) Transfer(ctx context.Context, params TransferParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.FromIdentity == nil || params.FromIdentity.handle == nil {
		return errors.New("from identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cToID := ffi.GoBytes32ToC(params.ToIdentity)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_transfer(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.Amount),
		cToID,
		params.FromIdentity.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// GetBalance gets token balance for an identity
func (t *TokenOperations) GetBalance(ctx context.Context, contractID ContractID, tokenPosition uint16, identityID IdentityID) (uint64, error) {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return 0, errors.New("SDK is closed")
	}

	cContractID := ffi.GoBytes32ToC(contractID)
	cIdentityID := ffi.GoBytes32ToC(identityID)

	var balance C.uint64_t
	result := C.dash_sdk_token_get_balance(
		t.sdk.handle,
		cContractID,
		C.uint16_t(tokenPosition),
		cIdentityID,
		&balance,
	)
	_, err := ffi.HandleResult(result)
	if err != nil {
		return 0, err
	}

	return uint64(balance), nil
}

// GetInfo gets token information
func (t *TokenOperations) GetInfo(ctx context.Context, contractID ContractID, tokenPosition uint16) (*TokenInfo, error) {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cContractID := ffi.GoBytes32ToC(contractID)
	
	result := C.dash_sdk_token_get_info(
		t.sdk.handle,
		cContractID,
		C.uint16_t(tokenPosition),
	)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, err
	}

	// Parse token info
	infoJSON := ffi.CStringToGoAndFree((*C.char)(data))
	var info TokenInfo
	if err := json.Unmarshal([]byte(infoJSON), &info); err != nil {
		return nil, fmt.Errorf("failed to parse token info: %w", err)
	}

	return &info, nil
}

// FreezeParams contains parameters for freezing tokens
type FreezeParams struct {
	ContractID       ContractID
	TokenPosition    uint16
	IdentityToFreeze IdentityID
	Owner            *Identity
	Settings         *PutSettings
}

// Freeze freezes tokens for an identity
func (t *TokenOperations) Freeze(ctx context.Context, params FreezeParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cIdentityID := ffi.GoBytes32ToC(params.IdentityToFreeze)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_freeze(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		cIdentityID,
		params.Owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// UnfreezeParams contains parameters for unfreezing tokens
type UnfreezeParams struct {
	ContractID         ContractID
	TokenPosition      uint16
	IdentityToUnfreeze IdentityID
	Owner              *Identity
	Settings           *PutSettings
}

// Unfreeze unfreezes tokens for an identity
func (t *TokenOperations) Unfreeze(ctx context.Context, params UnfreezeParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cIdentityID := ffi.GoBytes32ToC(params.IdentityToUnfreeze)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_unfreeze(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		cIdentityID,
		params.Owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// PurchaseParams contains parameters for purchasing tokens
type PurchaseParams struct {
	ContractID       ContractID
	TokenPosition    uint16
	Amount           uint64
	PriceOffer       uint64
	Purchaser        *Identity
	Settings         *PutSettings
}

// Purchase purchases tokens
func (t *TokenOperations) Purchase(ctx context.Context, params PurchaseParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Purchaser == nil || params.Purchaser.handle == nil {
		return errors.New("purchaser identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_purchase(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.Amount),
		C.uint64_t(params.PriceOffer),
		params.Purchaser.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// SetPriceParams contains parameters for setting token price
type SetPriceParams struct {
	ContractID    ContractID
	TokenPosition uint16
	NewPrice      uint64
	Owner         *Identity
	Settings      *PutSettings
}

// SetPrice sets the price for tokens
func (t *TokenOperations) SetPrice(ctx context.Context, params SetPriceParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_set_price(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.NewPrice),
		params.Owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// ClaimParams contains parameters for claiming tokens
type ClaimParams struct {
	ContractID       ContractID
	TokenPosition    uint16
	Amount           uint64
	DistributionType TokenDistributionType
	Claimer          *Identity
	Settings         *PutSettings
}

// TokenDistributionType represents the type of token distribution
type TokenDistributionType int

const (
	// TokenDistributionPreProgrammed represents pre-programmed distribution
	TokenDistributionPreProgrammed TokenDistributionType = iota
	// TokenDistributionPerpetual represents perpetual distribution
	TokenDistributionPerpetual
)

// Claim claims tokens from a distribution
func (t *TokenOperations) Claim(ctx context.Context, params ClaimParams) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if params.Claimer == nil || params.Claimer.handle == nil {
		return errors.New("claimer identity is required")
	}

	cContractID := ffi.GoBytes32ToC(params.ContractID)
	cSettings := convertPutSettings(params.Settings)

	result := C.dash_sdk_token_claim(
		t.sdk.handle,
		cContractID,
		C.uint16_t(params.TokenPosition),
		C.uint64_t(params.Amount),
		C.DashSDKTokenDistributionType(params.DistributionType),
		params.Claimer.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// GetAllocationInfo gets token allocation information for an identity
func (t *TokenOperations) GetAllocationInfo(ctx context.Context, contractID ContractID, tokenPosition uint16, identityID IdentityID) (*TokenAllocationInfo, error) {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cContractID := ffi.GoBytes32ToC(contractID)
	cIdentityID := ffi.GoBytes32ToC(identityID)

	result := C.dash_sdk_token_get_allocation_info(
		t.sdk.handle,
		cContractID,
		C.uint16_t(tokenPosition),
		cIdentityID,
	)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, err
	}

	// Parse allocation info
	infoJSON := ffi.CStringToGoAndFree((*C.char)(data))
	var info TokenAllocationInfo
	if err := json.Unmarshal([]byte(infoJSON), &info); err != nil {
		return nil, fmt.Errorf("failed to parse allocation info: %w", err)
	}

	return &info, nil
}

// DestroyFrozenFunds destroys frozen funds for an identity
func (t *TokenOperations) DestroyFrozenFunds(ctx context.Context, contractID ContractID, tokenPosition uint16, identityWithFrozenFunds IdentityID, owner *Identity, settings *PutSettings) error {
	t.sdk.mu.RLock()
	defer t.sdk.mu.RUnlock()

	if t.sdk.closed {
		return errors.New("SDK is closed")
	}

	if owner == nil || owner.handle == nil {
		return errors.New("owner identity is required")
	}

	cContractID := ffi.GoBytes32ToC(contractID)
	cIdentityID := ffi.GoBytes32ToC(identityWithFrozenFunds)
	cSettings := convertPutSettings(settings)

	result := C.dash_sdk_token_destroy_frozen_funds(
		t.sdk.handle,
		cContractID,
		C.uint16_t(tokenPosition),
		cIdentityID,
		owner.handle,
		cSettings,
	)
	_, err := ffi.HandleResult(result)
	
	return err
}

// TokenAllocationInfo contains token allocation information
type TokenAllocationInfo struct {
	IdentityID            string `json:"identityId"`
	TotalAllocation       uint64 `json:"totalAllocation"`
	RemainingAllocation   uint64 `json:"remainingAllocation"`
	ClaimedAmount         uint64 `json:"claimedAmount"`
	LastClaimAt           uint64 `json:"lastClaimAt,omitempty"`
	NextClaimAvailableAt  uint64 `json:"nextClaimAvailableAt,omitempty"`
}