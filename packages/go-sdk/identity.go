package dash

// #cgo CFLAGS: -I./internal/ffi
// #include "internal/ffi/dash_sdk_ffi.h"
// #include <stdlib.h>
import "C"
import (
	"context"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"unsafe"

	"github.com/btcsuite/btcutil/base58"
	"github.com/dashpay/platform/packages/go-sdk/internal/ffi"
)

// Identities provides identity-related operations
type Identities struct {
	sdk *SDK
}

// Identity represents a Dash Platform identity
type Identity struct {
	handle *ffi.IdentityHandle
	sdk    *SDK
	info   *IdentityInfo
}

// Create creates a new identity
func (i *Identities) Create(ctx context.Context) (*Identity, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	handle, err := ffi.CreateIdentity(i.sdk.handle)
	if err != nil {
		return nil, fmt.Errorf("failed to create identity: %w", err)
	}

	return &Identity{
		handle: handle,
		sdk:    i.sdk,
	}, nil
}

// Get fetches an identity by ID
func (i *Identities) Get(ctx context.Context, identityID string) (*Identity, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	// Validate and convert identity ID
	var id IdentityID
	if len(identityID) == 44 { // Base58 encoded
		decoded := base58.Decode(identityID)
		if len(decoded) != 32 {
			return nil, fmt.Errorf("invalid base58 identity ID")
		}
		copy(id[:], decoded)
	} else {
		// Try hex
		parsedID, err := NewIdentityIDFromString(identityID)
		if err != nil {
			return nil, fmt.Errorf("invalid identity ID: %w", err)
		}
		id = parsedID
	}

	cID := ffi.GoStringToC(id.String())
	defer C.free(unsafe.Pointer(cID))

	handle, err := ffi.FetchIdentity(i.sdk.handle, cID)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch identity: %w", err)
	}

	return &Identity{
		handle: handle,
		sdk:    i.sdk,
	}, nil
}

// GetByPublicKeyHash fetches an identity by public key hash
func (i *Identities) GetByPublicKeyHash(ctx context.Context, publicKeyHash PublicKeyHash) (*Identity, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cHash := ffi.GoBytes20ToC(publicKeyHash)
	handle, err := ffi.FetchIdentityByPublicKeyHash(i.sdk.handle, cHash)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch identity by public key hash: %w", err)
	}

	return &Identity{
		handle: handle,
		sdk:    i.sdk,
	}, nil
}

// GetBalance fetches the balance of an identity
func (i *Identities) GetBalance(ctx context.Context, identityID string) (uint64, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return 0, errors.New("SDK is closed")
	}

	cID := ffi.GoStringToC(identityID)
	defer C.free(unsafe.Pointer(cID))

	balance, err := ffi.FetchIdentityBalance(i.sdk.handle, cID)
	if err != nil {
		return 0, fmt.Errorf("failed to fetch identity balance: %w", err)
	}

	return balance, nil
}

// GetBalances fetches balances for multiple identities
func (i *Identities) GetBalances(ctx context.Context, identityIDs []string) (map[string]uint64, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	if len(identityIDs) == 0 {
		return make(map[string]uint64), nil
	}

	// Convert identity IDs to 32-byte arrays
	idArrays := make([][32]C.uint8_t, len(identityIDs))
	for idx, idStr := range identityIDs {
		// Parse identity ID (handle both hex and base58)
		var id IdentityID
		if len(idStr) == 44 { // Base58 encoded
			decoded := base58.Decode(idStr)
			if len(decoded) != 32 {
				return nil, fmt.Errorf("invalid base58 identity ID at index %d", idx)
			}
			copy(id[:], decoded)
		} else {
			// Try hex
			parsedID, err := NewIdentityIDFromString(idStr)
			if err != nil {
				return nil, fmt.Errorf("invalid identity ID at index %d: %w", idx, err)
			}
			id = parsedID
		}

		// Convert to C array
		for i := 0; i < 32; i++ {
			idArrays[idx][i] = C.uint8_t(id[i])
		}
	}

	// Call FFI function with array of identity IDs
	var idPtr *[32]C.uint8_t
	if len(idArrays) > 0 {
		idPtr = &idArrays[0]
	}
	
	result := C.dash_sdk_identities_fetch_balances(i.sdk.handle, idPtr, C.uintptr_t(len(idArrays)))
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch identity balances: %w", err)
	}

	// Parse the identity balance map
	balanceMap := (*C.DashSDKIdentityBalanceMap)(data)
	defer C.dash_sdk_identity_balance_map_free(balanceMap)

	balances := make(map[string]uint64)
	if balanceMap == nil || balanceMap.count == 0 {
		return balances, nil
	}

	// Convert C array to Go slice
	// Cast entries pointer to Go slice
	entries := (*[1 << 30]C.DashSDKIdentityBalanceEntry)(unsafe.Pointer(balanceMap.entries))[:balanceMap.count:balanceMap.count]
	
	for _, entry := range entries {
		// Convert identity ID to hex string
		var id [32]byte
		for i := 0; i < 32; i++ {
			id[i] = byte(entry.identity_id[i])
		}
		idStr := hex.EncodeToString(id[:])

		// Check if identity was found (balance != uint64 max)
		if entry.balance != ^uint64(0) {
			balances[idStr] = uint64(entry.balance)
		}
		// If balance is uint64 max, the identity was not found - we skip it
	}

	return balances, nil
}

// Identity methods

// GetInfo returns identity information
func (id *Identity) GetInfo() (*IdentityInfo, error) {
	if id.info != nil {
		return id.info, nil
	}

	cInfo := ffi.GetIdentityInfo(id.handle)
	if cInfo == nil {
		return nil, errors.New("failed to get identity info")
	}
	defer ffi.FreeIdentityInfo(cInfo)

	// Convert C info to Go struct
	info := &IdentityInfo{
		ID:      ffi.CStringToGo(cInfo.id),
		Balance: uint64(cInfo.balance),
	}

	// Parse public keys
	if cInfo.public_keys != nil {
		keysJSON := ffi.CStringToGo(cInfo.public_keys)
		if err := json.Unmarshal([]byte(keysJSON), &info.PublicKeys); err != nil {
			return nil, fmt.Errorf("failed to parse public keys: %w", err)
		}
	}

	// Parse contract bounds
	if cInfo.contract_bounds != nil {
		boundsJSON := ffi.CStringToGo(cInfo.contract_bounds)
		if err := json.Unmarshal([]byte(boundsJSON), &info.ContractBounds); err != nil {
			return nil, fmt.Errorf("failed to parse contract bounds: %w", err)
		}
	}

	id.info = info
	return info, nil
}

// GetID returns the identity ID
func (id *Identity) GetID() (string, error) {
	info, err := id.GetInfo()
	if err != nil {
		return "", err
	}
	return info.ID, nil
}

// GetBalance returns the identity balance
func (id *Identity) GetBalance() (uint64, error) {
	info, err := id.GetInfo()
	if err != nil {
		return 0, err
	}
	return info.Balance, nil
}

// RegisterName registers a name for the identity
func (id *Identity) RegisterName(ctx context.Context, name string, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	cName := ffi.GoStringToC(name)
	defer C.free(unsafe.Pointer(cName))

	cSettings := convertPutSettings(settings)
	err := C.dash_sdk_identity_register_name(id.sdk.handle, id.handle, cName, cSettings)
	
	return ffi.CErrorToGoError(err)
}

// ResolveName resolves a name to an identity
func (i *Identities) ResolveName(ctx context.Context, name string) (*Identity, error) {
	i.sdk.mu.RLock()
	defer i.sdk.mu.RUnlock()

	if i.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cName := ffi.GoStringToC(name)
	defer C.free(unsafe.Pointer(cName))

	result := C.dash_sdk_identity_resolve_name(i.sdk.handle, cName)
	handle, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve name: %w", err)
	}

	return &Identity{
		handle: (*ffi.IdentityHandle)(handle),
		sdk:    i.sdk,
	}, nil
}

// TopUp tops up an identity with credits
func (id *Identity) TopUp(ctx context.Context, amount uint64, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	cSettings := convertPutSettings(settings)
	result := C.dash_sdk_identity_topup(id.sdk.handle, id.handle, C.uint64_t(amount), cSettings)
	_, err := ffi.HandleResult(result)
	
	return err
}

// TransferCredits transfers credits to another identity
func (id *Identity) TransferCredits(ctx context.Context, toIdentityID IdentityID, amount uint64, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	cToID := ffi.GoBytes32ToC(toIdentityID)
	cSettings := convertPutSettings(settings)

	err := ffi.TransferCredits(id.sdk.handle, id.handle, cToID, C.uint64_t(amount), cSettings)
	return err
}

// Withdraw withdraws credits from the identity
func (id *Identity) Withdraw(ctx context.Context, amount uint64, toAddress string, settings *PutSettings) (*WithdrawalInfo, error) {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cAddress := ffi.GoStringToC(toAddress)
	defer C.free(unsafe.Pointer(cAddress))

	cSettings := convertPutSettings(settings)
	result := C.dash_sdk_identity_withdraw(id.sdk.handle, id.handle, C.uint64_t(amount), cAddress, cSettings)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to withdraw: %w", err)
	}

	// Parse withdrawal info
	infoJSON := ffi.CStringToGoAndFree((*C.char)(data))
	var info WithdrawalInfo
	if err := json.Unmarshal([]byte(infoJSON), &info); err != nil {
		return nil, fmt.Errorf("failed to parse withdrawal info: %w", err)
	}

	return &info, nil
}

// PutToInstantLock puts the identity to platform with instant lock
func (id *Identity) PutToInstantLock(ctx context.Context, proofPrivateKey []byte, fundingAmount uint64, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	if len(proofPrivateKey) != 32 {
		return errors.New("proof private key must be 32 bytes")
	}

	cKey, keyLen := ffi.GoBytesToC(proofPrivateKey)
	defer ffi.FreeC(cKey)

	cSettings := convertPutSettings(settings)
	err := ffi.PutIdentityToInstantLock(id.sdk.handle, id.handle, cKey, keyLen, C.uint64_t(fundingAmount), cSettings)
	
	return err
}

// PutToInstantLockAndWait puts the identity to platform with instant lock and waits
func (id *Identity) PutToInstantLockAndWait(ctx context.Context, proofPrivateKey []byte, fundingAmount uint64, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	if len(proofPrivateKey) != 32 {
		return errors.New("proof private key must be 32 bytes")
	}

	cKey, keyLen := ffi.GoBytesToC(proofPrivateKey)
	defer ffi.FreeC(cKey)

	cSettings := convertPutSettings(settings)
	err := ffi.PutIdentityToInstantLockAndWait(id.sdk.handle, id.handle, cKey, keyLen, C.uint64_t(fundingAmount), cSettings)
	
	return err
}

// PutToChainLock puts the identity to platform with chain lock
func (id *Identity) PutToChainLock(ctx context.Context, coreTxHeight uint32, fundingAmount uint64, assetLockProof []byte, settings *PutSettings) error {
	id.sdk.mu.RLock()
	defer id.sdk.mu.RUnlock()

	if id.sdk.closed {
		return errors.New("SDK is closed")
	}

	cProof, proofLen := ffi.GoBytesToC(assetLockProof)
	defer ffi.FreeC(cProof)

	cSettings := convertPutSettings(settings)
	err := ffi.PutIdentityToChainLock(id.sdk.handle, id.handle, C.uint32_t(coreTxHeight), C.uint64_t(fundingAmount), cProof, proofLen, cSettings)
	
	return err
}

// Close releases the identity handle
func (id *Identity) Close() error {
	if id.handle != nil {
		ffi.DestroyIdentityManual(id.handle)
		id.handle = nil
	}
	return nil
}

// WithdrawalInfo contains information about a withdrawal
type WithdrawalInfo struct {
	TransactionID string `json:"transactionId"`
	Amount        uint64 `json:"amount"`
	CoreFee       uint64 `json:"coreFee"`
	Status        string `json:"status"`
}

