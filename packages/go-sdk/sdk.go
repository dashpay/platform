package dash

// #cgo CFLAGS: -I./internal/ffi
// #cgo LDFLAGS: -L../../target/release -ldash_sdk_ffi -ldl -lpthread -lm
// #include "internal/ffi/dash_sdk_ffi.h"
// #include <stdlib.h>
import "C"
import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"runtime"
	"strings"
	"sync"
	"unsafe"

	"github.com/dashpay/platform/packages/go-sdk/internal/ffi"
)

// SDK represents the main Dash SDK instance
type SDK struct {
	handle    *ffi.SDKHandle
	config    *Config
	mu        sync.RWMutex
	closed    bool
	
	// Sub-modules
	identities    *Identities
	contracts     *Contracts
	documents     *Documents
}

// init initializes the SDK library
func init() {
	ffi.Init()
}

// Version returns the SDK version
func Version() string {
	return ffi.Version()
}

// NewSDK creates a new SDK instance with the given configuration
func NewSDK(config *Config) (*SDK, error) {
	if config == nil {
		config = DefaultConfig()
	}

	// Convert Go config to C config
	cConfig := &C.DashSDKConfig{
		network: C.DashSDKNetwork(config.Network),
	}

	// Set DAPI addresses if provided
	if len(config.DAPIAddresses) > 0 {
		addresses := strings.Join(config.DAPIAddresses, ",")
		cConfig.dapi_addresses = C.CString(addresses)
		defer C.free(unsafe.Pointer(cConfig.dapi_addresses))
	}

	// Set retry configuration
	cConfig.max_retries = C.uint32_t(config.MaxRetries)
	cConfig.retry_delay_ms = C.uint64_t(config.RetryDelay.Milliseconds())
	cConfig.ban_failed_address = C.bool(config.BanFailedAddress)

	// Set timeout configuration
	cConfig.connect_timeout_ms = C.uint64_t(config.ConnectTimeout.Milliseconds())
	cConfig.timeout_ms = C.uint64_t(config.RequestTimeout.Milliseconds())
	cConfig.wait_timeout_ms = C.uint64_t(config.WaitTimeout.Milliseconds())

	// Set identity configuration
	cConfig.identity_nonce_stale_time_s = C.uint64_t(config.IdentityNonceStaleTime.Seconds())

	// Set fee configuration
	cConfig.user_fee_increase = C.uint16_t(config.UserFeeIncrease)

	// Set debug options
	cConfig.allow_signing_with_any_security_level = C.bool(config.AllowSigningWithAnySecurityLevel)
	cConfig.allow_signing_with_any_purpose = C.bool(config.AllowSigningWithAnyPurpose)

	// Create SDK handle
	handle, err := ffi.CreateSDK(cConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to create SDK: %w", err)
	}

	sdk := &SDK{
		handle: handle,
		config: config,
	}

	// Initialize sub-modules
	sdk.identities = &Identities{sdk: sdk}
	sdk.contracts = &Contracts{sdk: sdk}
	sdk.documents = &Documents{sdk: sdk}

	// Set finalizer for automatic cleanup
	runtime.SetFinalizer(sdk, (*SDK).finalize)

	return sdk, nil
}

// NewMockSDK creates a new mock SDK instance for testing
func NewMockSDK() (*SDK, error) {
	handle, err := ffi.CreateSDKWithMock()
	if err != nil {
		return nil, fmt.Errorf("failed to create mock SDK: %w", err)
	}

	sdk := &SDK{
		handle: handle,
		config: DefaultConfig(),
	}

	// Initialize sub-modules
	sdk.identities = &Identities{sdk: sdk}
	sdk.contracts = &Contracts{sdk: sdk}
	sdk.documents = &Documents{sdk: sdk}

	runtime.SetFinalizer(sdk, (*SDK).finalize)

	return sdk, nil
}

// GetNetwork returns the network type of the SDK
func (s *SDK) GetNetwork() (Network, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if s.closed {
		return 0, errors.New("SDK is closed")
	}

	network, err := ffi.GetNetwork(s.handle)
	if err != nil {
		return 0, err
	}

	return Network(network), nil
}

// Close closes the SDK and releases resources
func (s *SDK) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.closed {
		return nil
	}

	s.closed = true
	runtime.SetFinalizer(s, nil)
	ffi.DestroySDKManual(s.handle)
	s.handle = nil

	return nil
}

// finalize is called by the garbage collector
func (s *SDK) finalize() {
	_ = s.Close()
}

// Identities returns the identities sub-module
func (s *SDK) Identities() *Identities {
	return s.identities
}

// Contracts returns the contracts sub-module
func (s *SDK) Contracts() *Contracts {
	return s.contracts
}

// Documents returns the documents sub-module
func (s *SDK) Documents() *Documents {
	return s.documents
}

// convertPutSettings converts Go PutSettings to C struct
func convertPutSettings(settings *PutSettings) *C.DashSDKPutSettings {
	if settings == nil {
		settings = DefaultPutSettings()
	}

	return &C.DashSDKPutSettings{
		connect_timeout_ms:                       C.uint64_t(settings.ConnectTimeout.Milliseconds()),
		timeout_ms:                               C.uint64_t(settings.RequestTimeout.Milliseconds()),
		retries:                                  C.uint32_t(settings.Retries),
		ban_failed_address:                       C.bool(settings.BanFailedAddress),
		identity_nonce_stale_time_s:              C.uint64_t(settings.IdentityNonceStaleTime.Seconds()),
		user_fee_increase:                        C.uint16_t(settings.UserFeeIncrease),
		allow_signing_with_any_security_level:    C.bool(settings.AllowSigningWithAnySecurityLevel),
		allow_signing_with_any_purpose:           C.bool(settings.AllowSigningWithAnyPurpose),
		wait_timeout_ms:                          C.uint64_t(settings.WaitTimeout.Milliseconds()),
	}
}

// convertTokenPaymentInfo converts Go TokenPaymentInfo to C struct
func convertTokenPaymentInfo(info *TokenPaymentInfo) *C.DashSDKTokenPaymentInfo {
	if info == nil {
		return nil
	}

	cInfo := &C.DashSDKTokenPaymentInfo{
		token_contract_position: C.uint16_t(info.TokenContractPosition),
		minimum_token_cost:      C.uint64_t(info.MinimumTokenCost),
		maximum_token_cost:      C.uint64_t(info.MaximumTokenCost),
		gas_fees_paid_by:        C.DashSDKGasFeesPaidBy(info.GasFeesPaidBy),
	}

	if info.PaymentTokenContractID != nil {
		cInfo.payment_token_contract_id = (*C.uint8_t)(unsafe.Pointer(&info.PaymentTokenContractID[0]))
	}

	return cInfo
}

// convertStateTransitionOptions converts Go options to C struct
func convertStateTransitionOptions(options *StateTransitionCreationOptions) *C.DashSDKStateTransitionCreationOptions {
	if options == nil {
		return nil
	}

	return &C.DashSDKStateTransitionCreationOptions{
		allow_signing_with_any_security_level: C.bool(options.AllowSigningWithAnySecurityLevel),
		allow_signing_with_any_purpose:        C.bool(options.AllowSigningWithAnyPurpose),
		batch_feature_version:                 C.uint16_t(options.BatchFeatureVersion),
		method_feature_version:                C.uint16_t(options.MethodFeatureVersion),
		base_feature_version:                  C.uint16_t(options.BaseFeatureVersion),
	}
}

// Platform Query Methods

// GetTotalCreditsInPlatform returns the total credits in the platform
func (s *SDK) GetTotalCreditsInPlatform(ctx context.Context) (uint64, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if s.closed {
		return 0, errors.New("SDK is closed")
	}

	var credits C.uint64_t
	result := C.dash_sdk_system_get_total_credits_in_platform(s.handle, &credits)
	if result.error != nil {
		return 0, ffi.CErrorToGoError(result.error)
	}

	return uint64(credits), nil
}

// GetPrefundedSpecializedBalance returns the prefunded specialized balance
func (s *SDK) GetPrefundedSpecializedBalance(ctx context.Context) (uint64, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if s.closed {
		return 0, errors.New("SDK is closed")
	}

	var balance C.uint64_t
	result := C.dash_sdk_system_get_prefunded_specialized_balance(s.handle, &balance)
	_, err := ffi.HandleResult(result)
	if err != nil {
		return 0, err
	}

	return uint64(balance), nil
}

// GetPathElements returns path elements for proof verification
func (s *SDK) GetPathElements(ctx context.Context, identityIDs []IdentityID, keys [][]byte) (map[string]interface{}, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	if s.closed {
		return nil, errors.New("SDK is closed")
	}

	// Convert identity IDs to JSON array
	idStrings := make([]string, len(identityIDs))
	for i, id := range identityIDs {
		idStrings[i] = id.String()
	}
	
	// Convert keys to hex strings
	keyStrings := make([]string, len(keys))
	for i, key := range keys {
		keyStrings[i] = fmt.Sprintf("%x", key)
	}

	// Create JSON request
	request := map[string]interface{}{
		"identityIds": idStrings,
		"keys":        keyStrings,
	}

	requestJSON, err := json.Marshal(request)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	cRequest := C.CString(string(requestJSON))
	defer C.free(unsafe.Pointer(cRequest))

	result := C.dash_sdk_system_get_path_elements(s.handle, cRequest)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, err
	}

	// Parse response
	responseStr := ffi.CStringToGoAndFree((*C.char)(data))
	var response map[string]interface{}
	if err := json.Unmarshal([]byte(responseStr), &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return response, nil
}