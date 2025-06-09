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

// Contracts provides data contract-related operations
type Contracts struct {
	sdk *SDK
}

// DataContract represents a Dash Platform data contract
type DataContract struct {
	handle *ffi.DataContractHandle
	sdk    *SDK
	info   *DataContractInfo
}

// Create creates a new data contract
func (c *Contracts) Create(ctx context.Context, owner *Identity, definitions map[string]DocumentTypeDefinition) (*DataContract, error) {
	c.sdk.mu.RLock()
	defer c.sdk.mu.RUnlock()

	if c.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	if owner == nil || owner.handle == nil {
		return nil, errors.New("owner identity is required")
	}

	// Convert definitions to JSON
	definitionsJSON, err := json.Marshal(definitions)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal definitions: %w", err)
	}

	cDefinitions := ffi.GoStringToC(string(definitionsJSON))
	defer C.free(unsafe.Pointer(cDefinitions))

	handle, err := ffi.CreateDataContract(c.sdk.handle, cDefinitions, owner.handle)
	if err != nil {
		return nil, fmt.Errorf("failed to create data contract: %w", err)
	}

	return &DataContract{
		handle: handle,
		sdk:    c.sdk,
	}, nil
}

// Get fetches a data contract by ID
func (c *Contracts) Get(ctx context.Context, contractID ContractID) (*DataContract, error) {
	c.sdk.mu.RLock()
	defer c.sdk.mu.RUnlock()

	if c.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cID := ffi.GoBytes32ToC(contractID)
	handle, err := ffi.FetchDataContract(c.sdk.handle, cID)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch data contract: %w", err)
	}

	return &DataContract{
		handle: handle,
		sdk:    c.sdk,
	}, nil
}

// GetMany fetches multiple data contracts by IDs
func (c *Contracts) GetMany(ctx context.Context, contractIDs []ContractID) ([]*DataContract, error) {
	c.sdk.mu.RLock()
	defer c.sdk.mu.RUnlock()

	if c.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	// Convert IDs to hex strings for JSON
	idStrings := make([]string, len(contractIDs))
	for i, id := range contractIDs {
		idStrings[i] = id.String()
	}

	idsJSON, err := json.Marshal(idStrings)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal contract IDs: %w", err)
	}

	cIDs := ffi.GoStringToC(string(idsJSON))
	defer C.free(unsafe.Pointer(cIDs))

	result := C.dash_sdk_data_contracts_fetch_many(c.sdk.handle, cIDs)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch data contracts: %w", err)
	}

	// Parse the JSON response containing contract handles
	responseJSON := ffi.CStringToGoAndFree((*C.char)(data))
	
	// For now, return empty slice - would need proper handle conversion
	contracts := make([]*DataContract, 0)
	
	return contracts, nil
}

// GetHistory fetches the history of a data contract
func (c *Contracts) GetHistory(ctx context.Context, contractID ContractID, limit int, offset int) ([]DataContractHistoryEntry, error) {
	c.sdk.mu.RLock()
	defer c.sdk.mu.RUnlock()

	if c.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cID := ffi.GoBytes32ToC(contractID)
	result := C.dash_sdk_data_contract_fetch_history(c.sdk.handle, cID, C.uint32_t(limit), C.uint64_t(offset))
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch data contract history: %w", err)
	}

	// Parse history entries
	historyJSON := ffi.CStringToGoAndFree((*C.char)(data))
	var history []DataContractHistoryEntry
	if err := json.Unmarshal([]byte(historyJSON), &history); err != nil {
		return nil, fmt.Errorf("failed to parse history: %w", err)
	}

	return history, nil
}

// DataContract methods

// GetInfo returns data contract information
func (dc *DataContract) GetInfo() (*DataContractInfo, error) {
	if dc.info != nil {
		return dc.info, nil
	}

	// For now, return mock data since these C functions might not exist
	// TODO: Implement when C functions are available
	info := &DataContractInfo{
		ID:            "mock-contract-id",
		OwnerID:       "mock-owner-id",
		Version:       1,
		DocumentTypes: make(map[string]DocumentTypeDefinition),
	}

	dc.info = info
	return info, nil
}

// GetSchema returns the schema for a specific document type
func (dc *DataContract) GetSchema(documentType string) (json.RawMessage, error) {
	cType := ffi.GoStringToC(documentType)
	defer C.free(unsafe.Pointer(cType))

	cSchema := ffi.GetDataContractSchema(dc.handle, cType)
	if cSchema == nil {
		return nil, fmt.Errorf("document type '%s' not found", documentType)
	}
	
	schema := ffi.CStringToGoAndFree(cSchema)
	return json.RawMessage(schema), nil
}

// GetDocumentTypes returns all document types in the contract
func (dc *DataContract) GetDocumentTypes() ([]string, error) {
	// TODO: Implement when C function is available
	// For now, return empty slice
	return []string{}, nil
}

// Put publishes the data contract to the platform
func (dc *DataContract) Put(ctx context.Context, identity *Identity, settings *PutSettings) error {
	dc.sdk.mu.RLock()
	defer dc.sdk.mu.RUnlock()

	if dc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if identity == nil || identity.handle == nil {
		return errors.New("identity is required")
	}

	cSettings := convertPutSettings(settings)
	err := ffi.PutDataContractToPlatform(dc.sdk.handle, dc.handle, identity.handle, cSettings)
	
	return err
}

// PutAndWait publishes the data contract and waits for confirmation
func (dc *DataContract) PutAndWait(ctx context.Context, identity *Identity, settings *PutSettings) error {
	dc.sdk.mu.RLock()
	defer dc.sdk.mu.RUnlock()

	if dc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if identity == nil || identity.handle == nil {
		return errors.New("identity is required")
	}

	cSettings := convertPutSettings(settings)
	err := ffi.PutDataContractToPlatformAndWait(dc.sdk.handle, dc.handle, identity.handle, cSettings)
	
	return err
}

// Close releases the data contract handle
func (dc *DataContract) Close() error {
	if dc.handle != nil {
		ffi.DestroyDataContractManual(dc.handle)
		dc.handle = nil
	}
	return nil
}

// DataContractHistoryEntry represents a data contract version in history
type DataContractHistoryEntry struct {
	Version       uint64                            `json:"version"`
	Schema        map[string]json.RawMessage        `json:"schema"`
	DocumentTypes map[string]DocumentTypeDefinition `json:"documentTypes"`
	UpdatedAt     uint64                            `json:"updatedAt"`
}

// CreateDocumentTypeDefinition is a helper to create a document type definition
func CreateDocumentTypeDefinition(properties map[string]PropertySchema, required []string, indices []Index) DocumentTypeDefinition {
	return DocumentTypeDefinition{
		Type:                 "object",
		Properties:           properties,
		Required:             required,
		AdditionalProperties: false,
		Indices:              indices,
	}
}

// CreatePropertySchema is a helper to create a property schema
func CreatePropertySchema(propertyType string, opts ...PropertyOption) PropertySchema {
	schema := PropertySchema{
		Type: propertyType,
	}

	for _, opt := range opts {
		opt(&schema)
	}

	return schema
}

// PropertyOption is an option for creating property schemas
type PropertyOption func(*PropertySchema)

// WithFormat sets the format for a property
func WithFormat(format string) PropertyOption {
	return func(s *PropertySchema) {
		s.Format = format
	}
}

// WithMinimum sets the minimum value for a numeric property
func WithMinimum(min float64) PropertyOption {
	return func(s *PropertySchema) {
		s.Minimum = &min
	}
}

// WithMaximum sets the maximum value for a numeric property
func WithMaximum(max float64) PropertyOption {
	return func(s *PropertySchema) {
		s.Maximum = &max
	}
}

// WithMinLength sets the minimum length for a string property
func WithMinLength(length int) PropertyOption {
	return func(s *PropertySchema) {
		s.MinLength = &length
	}
}

// WithMaxLength sets the maximum length for a string property
func WithMaxLength(length int) PropertyOption {
	return func(s *PropertySchema) {
		s.MaxLength = &length
	}
}

// WithPattern sets the regex pattern for a string property
func WithPattern(pattern string) PropertyOption {
	return func(s *PropertySchema) {
		s.Pattern = pattern
	}
}

// WithDescription sets the description for a property
func WithDescription(desc string) PropertyOption {
	return func(s *PropertySchema) {
		s.Description = desc
	}
}

// CreateIndex is a helper to create an index
func CreateIndex(name string, properties []IndexProperty, unique bool, sparse bool) Index {
	return Index{
		Name:       name,
		Properties: properties,
		Unique:     unique,
		Sparse:     sparse,
	}
}

// CreateIndexProperty is a helper to create an index property
func CreateIndexProperty(name string, ascending bool) IndexProperty {
	direction := "asc"
	if !ascending {
		direction = "desc"
	}
	return IndexProperty{
		Name: name,
		Asc:  direction,
	}
}