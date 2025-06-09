package ffi

// #cgo CFLAGS: -I.
// #cgo LDFLAGS: -L../../../target/release -ldash_sdk_ffi -ldl -lpthread -lm
// #include "dash_sdk_ffi.h"
// #include <stdlib.h>
import "C"
import (
	"errors"
	"fmt"
	"runtime"
	"unsafe"
)

// Error codes
const (
	Success            = C.Success
	InvalidParameter   = C.InvalidParameter
	InvalidState       = C.InvalidState
	NetworkError       = C.NetworkError
	SerializationError = C.SerializationError
	ProtocolError      = C.ProtocolError
	CryptoError        = C.CryptoError
	NotFound           = C.NotFound
	Timeout            = C.Timeout
	NotImplemented     = C.NotImplemented
	InternalError      = C.InternalError
)

// Network types
const (
	NetworkMainnet = C.Mainnet
	NetworkTestnet = C.Testnet
	NetworkDevnet  = C.Devnet
	NetworkLocal   = C.Local
)

// Result data types
const (
	DataTypeNone               = C.None
	DataTypeString             = C.String
	DataTypeBinaryData         = C.BinaryData
	DataTypeIdentityHandle     = C.IdentityHandle
	DataTypeDocumentHandle     = C.DocumentHandle
	DataTypeDataContractHandle = C.DataContractHandle
	DataTypeIdentityBalanceMap = C.IdentityBalanceMap
)

// Handle types
type (
	SDKHandle              = C.SDKHandle
	IdentityHandle         = C.IdentityHandle
	DocumentHandle         = C.DocumentHandle
	DataContractHandle     = C.DataContractHandle
	SignerHandle           = C.SignerHandle
	IdentityPublicKeyHandle = C.IdentityPublicKeyHandle
)

// Init initializes the SDK (sets up panic handlers)
func Init() {
	C.dash_sdk_init()
}

// Version returns the SDK version
func Version() string {
	cStr := C.dash_sdk_version()
	return C.GoString(cStr)
}

// CErrorToGoError converts C error to Go error
func CErrorToGoError(cErr *C.DashSDKError) error {
	if cErr == nil {
		return nil
	}
	defer C.dash_sdk_error_free(cErr)
	
	if cErr.code == Success {
		return nil
	}
	
	message := "unknown error"
	if cErr.message != nil {
		message = C.GoString(cErr.message)
	}
	
	return fmt.Errorf("SDK error %d: %s", cErr.code, message)
}

// HandleResult handles C results
func HandleResult(result C.DashSDKResult) (unsafe.Pointer, error) {
	if result.error != nil {
		return nil, CErrorToGoError(result.error)
	}
	return result.data, nil
}

// FreeString frees a C string allocated by the SDK
func FreeString(s *C.char) {
	C.dash_sdk_string_free(s)
}

// FreeBinaryData frees binary data allocated by the SDK
func FreeBinaryData(data unsafe.Pointer) {
	C.dash_sdk_binary_data_free(data)
}

// FreeBytes frees bytes allocated by the SDK
func FreeBytes(data unsafe.Pointer) {
	C.dash_sdk_bytes_free(data)
}

// SDK Creation and Management

// CreateSDK creates a new SDK instance
func CreateSDK(config *C.DashSDKConfig) (*SDKHandle, error) {
	result := C.dash_sdk_create(config)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	sdkHandle := (*SDKHandle)(handle)
	// Set up finalizer for automatic cleanup
	runtime.SetFinalizer(sdkHandle, destroySDK)
	
	return sdkHandle, nil
}

// CreateSDKWithMock creates a mock SDK instance for testing
func CreateSDKWithMock() (*SDKHandle, error) {
	result := C.dash_sdk_create_handle_with_mock()
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	sdkHandle := (*SDKHandle)(handle)
	runtime.SetFinalizer(sdkHandle, destroySDK)
	
	return sdkHandle, nil
}

// GetNetwork returns the network type of the SDK
func GetNetwork(sdk *SDKHandle) (C.DashSDKNetwork, error) {
	if sdk == nil {
		return 0, errors.New("nil SDK handle")
	}
	
	var network C.DashSDKNetwork
	var err *C.DashSDKError
	err = C.dash_sdk_get_network(sdk, &network)
	
	if err != nil {
		return 0, CErrorToGoError(err)
	}
	
	return network, nil
}

// DestroySDK destroys an SDK instance
func destroySDK(sdk *SDKHandle) {
	if sdk != nil {
		C.dash_sdk_destroy(sdk)
	}
}

// DestroySDKManual allows manual destruction of SDK
func DestroySDKManual(sdk *SDKHandle) {
	if sdk != nil {
		runtime.SetFinalizer(sdk, nil)
		C.dash_sdk_destroy(sdk)
	}
}

// Identity Management

// CreateIdentity creates a new identity
func CreateIdentity(sdk *SDKHandle) (*IdentityHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_create(sdk)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	identityHandle := (*IdentityHandle)(handle)
	runtime.SetFinalizer(identityHandle, destroyIdentity)
	
	return identityHandle, nil
}

// FetchIdentity fetches an identity by ID
func FetchIdentity(sdk *SDKHandle, identityID *C.char) (*IdentityHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_fetch(sdk, identityID)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	identityHandle := (*IdentityHandle)(handle)
	runtime.SetFinalizer(identityHandle, destroyIdentity)
	
	return identityHandle, nil
}

// FetchIdentityByPublicKeyHash fetches an identity by public key hash
func FetchIdentityByPublicKeyHash(sdk *SDKHandle, publicKeyHash *[20]C.uint8_t) (*IdentityHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_fetch_by_public_key_hash(sdk, publicKeyHash)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	identityHandle := (*IdentityHandle)(handle)
	runtime.SetFinalizer(identityHandle, destroyIdentity)
	
	return identityHandle, nil
}

// GetIdentityInfo gets identity information
func GetIdentityInfo(identity *IdentityHandle) *C.DashSDKIdentityInfo {
	if identity == nil {
		return nil
	}
	return C.dash_sdk_identity_get_info(identity)
}

// FreeIdentityInfo frees identity info structure
func FreeIdentityInfo(info *C.DashSDKIdentityInfo) {
	if info != nil {
		C.dash_sdk_identity_info_free(info)
	}
}

// DestroyIdentity destroys an identity handle
func destroyIdentity(identity *IdentityHandle) {
	if identity != nil {
		C.dash_sdk_identity_destroy(identity)
	}
}

// DestroyIdentityManual allows manual destruction of identity
func DestroyIdentityManual(identity *IdentityHandle) {
	if identity != nil {
		runtime.SetFinalizer(identity, nil)
		C.dash_sdk_identity_destroy(identity)
	}
}

// Document Management

// CreateDocument creates a new document
func CreateDocument(sdk *SDKHandle, params *C.DashSDKDocumentCreateParams) (*DocumentHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_create(sdk, params)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	documentHandle := (*DocumentHandle)(handle)
	runtime.SetFinalizer(documentHandle, destroyDocumentHandle)
	
	return documentHandle, nil
}

// FetchDocument fetches a document by ID
func FetchDocument(sdk *SDKHandle, contractID *[32]C.uint8_t, documentType *C.char, documentID *[32]C.uint8_t) (*DocumentHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_fetch(sdk, contractID, documentType, documentID)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	documentHandle := (*DocumentHandle)(handle)
	runtime.SetFinalizer(documentHandle, destroyDocumentHandle)
	
	return documentHandle, nil
}

// SearchDocuments searches for documents
func SearchDocuments(sdk *SDKHandle, dataContract *DataContractHandle, documentType *C.char, query *C.char) (*C.char, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_search(sdk, dataContract, documentType, query)
	data, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	return (*C.char)(data), nil
}

// GetDocumentInfo gets document information
func GetDocumentInfo(document *DocumentHandle) *C.DashSDKDocumentInfo {
	if document == nil {
		return nil
	}
	return C.dash_sdk_document_get_info(document)
}

// FreeDocumentInfo frees document info structure
func FreeDocumentInfo(info *C.DashSDKDocumentInfo) {
	if info != nil {
		C.dash_sdk_document_info_free(info)
	}
}

// DestroyDocument destroys a document on platform
func DestroyDocument(sdk *SDKHandle, document *DocumentHandle, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	err := C.dash_sdk_document_destroy(sdk, document, settings)
	return CErrorToGoError(err)
}

// Additional FFI bindings that might be missing from header

// DestroyDocumentHandle destroys a document handle
func destroyDocumentHandle(document *DocumentHandle) {
	if document != nil {
		C.dash_sdk_document_handle_destroy(document)
	}
}

// DestroyDocumentHandleManual allows manual destruction of document handle
func DestroyDocumentHandleManual(document *DocumentHandle) {
	if document != nil {
		runtime.SetFinalizer(document, nil)
		C.dash_sdk_document_handle_destroy(document)
	}
}

// SetDocumentProperties sets all properties on a document
func SetDocumentProperties(document *DocumentHandle, propertiesJSON string) error {
	if document == nil {
		return errors.New("nil document handle")
	}
	
	cJSON := C.CString(propertiesJSON)
	defer C.free(unsafe.Pointer(cJSON))
	
	cErr := C.dash_sdk_document_set_properties(document, cJSON)
	return CErrorToGoError(cErr)
}

// SetDocumentProperty sets a single property on a document
func SetDocumentProperty(document *DocumentHandle, path string, valueJSON string) error {
	if document == nil {
		return errors.New("nil document handle")
	}
	
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))
	
	cValue := C.CString(valueJSON)
	defer C.free(unsafe.Pointer(cValue))
	
	cErr := C.dash_sdk_document_set(document, cPath, cValue)
	return CErrorToGoError(cErr)
}

// RemoveDocumentProperty removes a property from a document
func RemoveDocumentProperty(document *DocumentHandle, path string) error {
	if document == nil {
		return errors.New("nil document handle")
	}
	
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))
	
	cErr := C.dash_sdk_document_remove(document, cPath)
	return CErrorToGoError(cErr)
}

// Data Contract Management

// CreateDataContract creates a new data contract
func CreateDataContract(sdk *SDKHandle, definitionsJSON *C.char, ownerIdentity *IdentityHandle) (*DataContractHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_data_contract_create(sdk, definitionsJSON, ownerIdentity)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	contractHandle := (*DataContractHandle)(handle)
	runtime.SetFinalizer(contractHandle, destroyDataContract)
	
	return contractHandle, nil
}

// FetchDataContract fetches a data contract by ID
func FetchDataContract(sdk *SDKHandle, contractID *[32]C.uint8_t) (*DataContractHandle, error) {
	if sdk == nil {
		return nil, errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_data_contract_fetch(sdk, contractID)
	handle, err := HandleResult(result)
	if err != nil {
		return nil, err
	}
	
	contractHandle := (*DataContractHandle)(handle)
	runtime.SetFinalizer(contractHandle, destroyDataContract)
	
	return contractHandle, nil
}

// GetDataContractSchema gets the schema for a document type
func GetDataContractSchema(contract *DataContractHandle, documentType *C.char) *C.char {
	if contract == nil {
		return nil
	}
	return C.dash_sdk_data_contract_get_schema(contract, documentType)
}

// DestroyDataContract destroys a data contract handle
func destroyDataContract(contract *DataContractHandle) {
	if contract != nil {
		C.dash_sdk_data_contract_destroy(contract)
	}
}

// DestroyDataContractManual allows manual destruction of data contract
func DestroyDataContractManual(contract *DataContractHandle) {
	if contract != nil {
		runtime.SetFinalizer(contract, nil)
		C.dash_sdk_data_contract_destroy(contract)
	}
}

// Platform Operations

// PutIdentityToInstantLock puts identity to platform with instant lock
func PutIdentityToInstantLock(sdk *SDKHandle, identity *IdentityHandle, proofPrivateKey unsafe.Pointer, proofPrivateKeyLen C.size_t, fundingAmount C.uint64_t, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_put_to_platform_with_instant_lock(sdk, identity, proofPrivateKey, proofPrivateKeyLen, fundingAmount, settings)
	_, err := HandleResult(result)
	return err
}

// PutIdentityToChainLock puts identity to platform with chain lock
func PutIdentityToChainLock(sdk *SDKHandle, identity *IdentityHandle, coreTxHeight C.uint32_t, fundingAmount C.uint64_t, assetLockProof unsafe.Pointer, assetLockProofLen C.size_t, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_put_to_platform_with_chain_lock(sdk, identity, coreTxHeight, fundingAmount, assetLockProof, assetLockProofLen, settings)
	_, err := HandleResult(result)
	return err
}

// PutDocumentToPlatform puts a document to platform
func PutDocumentToPlatform(sdk *SDKHandle, document *DocumentHandle, documentType *C.char, dataContract *DataContractHandle, putSettings *C.DashSDKPutSettings, signedBy *IdentityHandle, paymentInfo *C.DashSDKTokenPaymentInfo) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_put_to_platform(sdk, document, documentType, dataContract, putSettings, signedBy, paymentInfo)
	_, err := HandleResult(result)
	return err
}

// ReplaceDocumentOnPlatform replaces a document on platform
func ReplaceDocumentOnPlatform(sdk *SDKHandle, document *DocumentHandle, dataContract *DataContractHandle, putSettings *C.DashSDKPutSettings, signedBy *IdentityHandle, paymentInfo *C.DashSDKTokenPaymentInfo) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_replace_on_platform(sdk, document, dataContract, putSettings, signedBy, paymentInfo)
	_, err := HandleResult(result)
	return err
}

// PutDataContractToPlatform puts a data contract to platform
func PutDataContractToPlatform(sdk *SDKHandle, contract *DataContractHandle, identity *IdentityHandle, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_data_contract_put_to_platform(sdk, contract, identity, settings)
	_, err := HandleResult(result)
	return err
}

// Balance Operations

// FetchIdentityBalance fetches identity balance
func FetchIdentityBalance(sdk *SDKHandle, identityID *C.char) (uint64, error) {
	if sdk == nil {
		return 0, errors.New("nil SDK handle")
	}
	
	var balance C.uint64_t
	result := C.dash_sdk_identity_fetch_balance(sdk, identityID, &balance)
	_, err := HandleResult(result)
	if err != nil {
		return 0, err
	}
	
	return uint64(balance), nil
}

// Transfer Operations

// TransferCredits transfers credits between identities
func TransferCredits(sdk *SDKHandle, identity *IdentityHandle, toIdentityID *[32]C.uint8_t, amount C.uint64_t, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_transfer_credits(sdk, identity, toIdentityID, amount, settings)
	_, err := HandleResult(result)
	return err
}

// Wait versions of operations

// PutIdentityToInstantLockAndWait puts identity and waits for confirmation
func PutIdentityToInstantLockAndWait(sdk *SDKHandle, identity *IdentityHandle, proofPrivateKey unsafe.Pointer, proofPrivateKeyLen C.size_t, fundingAmount C.uint64_t, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_identity_put_to_platform_with_instant_lock_and_wait(sdk, identity, proofPrivateKey, proofPrivateKeyLen, fundingAmount, settings)
	_, err := HandleResult(result)
	return err
}

// PutDocumentToPlatformAndWait puts a document and waits for confirmation
func PutDocumentToPlatformAndWait(sdk *SDKHandle, document *DocumentHandle, documentType *C.char, dataContract *DataContractHandle, putSettings *C.DashSDKPutSettings, signedBy *IdentityHandle, paymentInfo *C.DashSDKTokenPaymentInfo) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_document_put_to_platform_and_wait(sdk, document, documentType, dataContract, putSettings, signedBy, paymentInfo)
	_, err := HandleResult(result)
	return err
}

// PutDataContractToPlatformAndWait puts a data contract and waits for confirmation
func PutDataContractToPlatformAndWait(sdk *SDKHandle, contract *DataContractHandle, identity *IdentityHandle, settings *C.DashSDKPutSettings) error {
	if sdk == nil {
		return errors.New("nil SDK handle")
	}
	
	result := C.dash_sdk_data_contract_put_to_platform_and_wait(sdk, contract, identity, settings)
	_, err := HandleResult(result)
	return err
}