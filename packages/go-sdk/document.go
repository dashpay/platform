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

// getNumberField extracts a numeric field from a map, handling different JSON number types
func getNumberField(data map[string]interface{}, field string) (float64, bool) {
	value, ok := data[field]
	if !ok {
		return 0, false
	}

	switch v := value.(type) {
	case float64:
		return v, true
	case float32:
		return float64(v), true
	case int:
		return float64(v), true
	case int64:
		return float64(v), true
	case uint64:
		return float64(v), true
	case json.Number:
		f, err := v.Float64()
		if err != nil {
			return 0, false
		}
		return f, true
	default:
		return 0, false
	}
}

// Documents provides document-related operations
type Documents struct {
	sdk *SDK
}

// Document represents a Dash Platform document
type Document struct {
	handle       *ffi.DocumentHandle
	sdk          *SDK
	info         *DocumentInfo
	documentType string
	dataContract *DataContract
}

// CreateParams contains parameters for creating a document
type CreateParams struct {
	DataContract *DataContract
	DocumentType string
	Owner        *Identity
	Properties   map[string]interface{}
}

// Create creates a new document
func (d *Documents) Create(ctx context.Context, params CreateParams) (*Document, error) {
	d.sdk.mu.RLock()
	defer d.sdk.mu.RUnlock()

	if d.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	if params.DataContract == nil || params.DataContract.handle == nil {
		return nil, errors.New("data contract is required")
	}

	if params.Owner == nil || params.Owner.handle == nil {
		return nil, errors.New("owner identity is required")
	}

	if params.DocumentType == "" {
		return nil, errors.New("document type is required")
	}

	// Convert properties to JSON
	propertiesJSON, err := json.Marshal(params.Properties)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal properties: %w", err)
	}

	cType := ffi.GoStringToC(params.DocumentType)
	defer C.free(unsafe.Pointer(cType))

	cProperties := ffi.GoStringToC(string(propertiesJSON))
	defer C.free(unsafe.Pointer(cProperties))

	cParams := &C.DashSDKDocumentCreateParams{
		data_contract_handle:  params.DataContract.handle,
		document_type:         cType,
		owner_identity_handle: params.Owner.handle,
		properties_json:       cProperties,
	}

	handle, err := ffi.CreateDocument(d.sdk.handle, cParams)
	if err != nil {
		return nil, fmt.Errorf("failed to create document: %w", err)
	}

	return &Document{
		handle:       handle,
		sdk:          d.sdk,
		documentType: params.DocumentType,
		dataContract: params.DataContract,
	}, nil
}

// Get fetches a document by ID
func (d *Documents) Get(ctx context.Context, contractID ContractID, documentType string, documentID DocumentID) (*Document, error) {
	d.sdk.mu.RLock()
	defer d.sdk.mu.RUnlock()

	if d.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	cContractID := ffi.GoBytes32ToC(contractID)
	cType := ffi.GoStringToC(documentType)
	defer C.free(unsafe.Pointer(cType))
	cDocumentID := ffi.GoBytes32ToC(documentID)

	handle, err := ffi.FetchDocument(d.sdk.handle, cContractID, cType, cDocumentID)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch document: %w", err)
	}

	// Also fetch the data contract for context
	contract, err := d.sdk.Contracts().Get(ctx, contractID)
	if err != nil {
		// Continue without contract, it's optional
		contract = nil
	}

	return &Document{
		handle:       handle,
		sdk:          d.sdk,
		documentType: documentType,
		dataContract: contract,
	}, nil
}

// Search searches for documents
func (d *Documents) Search(ctx context.Context, contract *DataContract, documentType string, query DocumentQuery) ([]*Document, error) {
	d.sdk.mu.RLock()
	defer d.sdk.mu.RUnlock()

	if d.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	if contract == nil || contract.handle == nil {
		return nil, errors.New("data contract is required")
	}

	queryJSON, err := json.Marshal(query)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal query: %w", err)
	}

	cType := ffi.GoStringToC(documentType)
	defer C.free(unsafe.Pointer(cType))

	cQuery := ffi.GoStringToC(string(queryJSON))
	defer C.free(unsafe.Pointer(cQuery))

	resultJSON, err := ffi.SearchDocuments(d.sdk.handle, contract.handle, cType, cQuery)
	if err != nil {
		return nil, fmt.Errorf("failed to search documents: %w", err)
	}
	defer ffi.FreeString(resultJSON)

	// Parse search results
	type searchResponse struct {
		Documents  []json.RawMessage `json:"documents"`
		TotalCount int               `json:"total_count"`
	}

	var response searchResponse
	if err := json.Unmarshal([]byte(ffi.CStringToGo(resultJSON)), &response); err != nil {
		return nil, fmt.Errorf("failed to parse search results: %w", err)
	}

	// Create document objects from the results
	documents := make([]*Document, 0, len(response.Documents))
	
	for _, docJSON := range response.Documents {
		// Parse document data
		var docData map[string]interface{}
		if err := json.Unmarshal(docJSON, &docData); err != nil {
			return nil, fmt.Errorf("failed to parse document data: %w", err)
		}

		// Create a document object
		// Note: Search results don't include handles, so we create documents without handles
		// These documents can be used for reading but not for operations that require a handle
		doc := &Document{
			handle:       nil, // No handle from search results
			sdk:          d.sdk,
			documentType: documentType,
			dataContract: contract,
			info:         nil, // Will be populated on first access
		}

		// Extract common document fields if present
		docInfo := &DocumentInfo{
			DocumentType:   documentType,
			DataContractID: "", // Will be set from contract if available
			Data:           docData,
		}

		// Try to extract standard fields
		if id, ok := docData["$id"].(string); ok {
			docInfo.ID = id
		}
		if ownerID, ok := docData["$ownerId"].(string); ok {
			docInfo.OwnerID = ownerID
		}
		if revision, ok := getNumberField(docData, "$revision"); ok {
			docInfo.Revision = uint64(revision)
		}
		if createdAt, ok := getNumberField(docData, "$createdAt"); ok {
			docInfo.CreatedAt = uint64(createdAt)
		}
		if updatedAt, ok := getNumberField(docData, "$updatedAt"); ok {
			docInfo.UpdatedAt = uint64(updatedAt)
		}

		// Set contract ID from the contract parameter
		if contract != nil {
			if contractInfo, err := contract.GetInfo(); err == nil {
				docInfo.DataContractID = contractInfo.ID
			}
		}

		doc.info = docInfo
		documents = append(documents, doc)
	}
	
	return documents, nil
}

// Document methods

// HasHandle returns true if the document has a handle (can be used for write operations)
func (doc *Document) HasHandle() bool {
	return doc.handle != nil
}

// requireHandle returns an error if the document doesn't have a handle
func (doc *Document) requireHandle(operation string) error {
	if doc.handle == nil {
		return fmt.Errorf("cannot %s document without handle - document was created from search results", operation)
	}
	return nil
}

// GetInfo returns document information
func (doc *Document) GetInfo() (*DocumentInfo, error) {
	if doc.info != nil {
		return doc.info, nil
	}

	// If document has no handle (e.g., from search results), we can't fetch additional info
	if doc.handle == nil {
		return nil, errors.New("document has no handle - created from search results")
	}

	cInfo := ffi.GetDocumentInfo(doc.handle)
	if cInfo == nil {
		return nil, errors.New("failed to get document info")
	}
	defer ffi.FreeDocumentInfo(cInfo)

	// Parse document data
	var data map[string]interface{}
	if cInfo.properties_json != nil {
		dataJSON := ffi.CStringToGo(cInfo.properties_json)
		if err := json.Unmarshal([]byte(dataJSON), &data); err != nil {
			return nil, fmt.Errorf("failed to parse document data: %w", err)
		}
	}

	info := &DocumentInfo{
		ID:             ffi.CStringToGo(cInfo.id),
		OwnerID:        ffi.CStringToGo(cInfo.owner_id),
		DataContractID: ffi.CStringToGo(cInfo.data_contract_id),
		DocumentType:   ffi.CStringToGo(cInfo.document_type),
		Revision:       uint64(cInfo.revision),
		CreatedAt:      uint64(cInfo.created_at),
		UpdatedAt:      uint64(cInfo.updated_at),
		Data:           data,
	}

	doc.info = info
	return info, nil
}

// GetID returns the document ID
func (doc *Document) GetID() (string, error) {
	info, err := doc.GetInfo()
	if err != nil {
		return "", err
	}
	return info.ID, nil
}

// GetData returns the document data
func (doc *Document) GetData() (map[string]interface{}, error) {
	info, err := doc.GetInfo()
	if err != nil {
		return nil, err
	}
	return info.Data, nil
}

// Set sets a field value in the document
func (doc *Document) Set(field string, value interface{}) error {
	info, err := doc.GetInfo()
	if err != nil {
		return err
	}

	if info.Data == nil {
		info.Data = make(map[string]interface{})
	}

	info.Data[field] = value
	doc.info = nil // Clear cached info
	
	// Update the document handle with new data
	return doc.updateHandle()
}

// Get gets a field value from the document
func (doc *Document) Get(field string) (interface{}, bool) {
	info, err := doc.GetInfo()
	if err != nil {
		return nil, false
	}

	value, exists := info.Data[field]
	return value, exists
}

// SetProperty sets a single property using a path (lodash-style notation)
func (doc *Document) SetProperty(path string, value interface{}) error {
	if err := doc.requireHandle("set property"); err != nil {
		return err
	}
	
	// Serialize the value to JSON
	valueJSON, err := json.Marshal(value)
	if err != nil {
		return fmt.Errorf("failed to serialize value: %w", err)
	}
	
	// Set the property using the FFI function
	if err := ffi.SetDocumentProperty(doc.handle, path, string(valueJSON)); err != nil {
		return fmt.Errorf("failed to set property: %w", err)
	}
	
	// Clear cached info to force reload on next access
	doc.info = nil
	return nil
}

// RemoveProperty removes a property using a path (lodash-style notation)
func (doc *Document) RemoveProperty(path string) error {
	if err := doc.requireHandle("remove property"); err != nil {
		return err
	}
	
	// Remove the property using the FFI function
	if err := ffi.RemoveDocumentProperty(doc.handle, path); err != nil {
		return fmt.Errorf("failed to remove property: %w", err)
	}
	
	// Clear cached info to force reload on next access
	doc.info = nil
	return nil
}

// updateHandle updates the document handle with current data
func (doc *Document) updateHandle() error {
	if err := doc.requireHandle("update"); err != nil {
		return err
	}
	
	// Get current document data
	info, err := doc.GetInfo()
	if err != nil {
		return err
	}
	
	// Serialize the data to JSON
	dataJSON, err := json.Marshal(info.Data)
	if err != nil {
		return fmt.Errorf("failed to serialize document data: %w", err)
	}
	
	// Update the document properties using the FFI function
	if err := ffi.SetDocumentProperties(doc.handle, string(dataJSON)); err != nil {
		return fmt.Errorf("failed to update document properties: %w", err)
	}
	
	// Clear cached info to force reload on next access
	doc.info = nil
	return nil
}

// Put publishes the document to the platform
func (doc *Document) Put(ctx context.Context, signedBy *Identity, settings *PutSettings, paymentInfo *TokenPaymentInfo) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("put"); err != nil {
		return err
	}

	if signedBy == nil || signedBy.handle == nil {
		return errors.New("signing identity is required")
	}

	if doc.dataContract == nil || doc.dataContract.handle == nil {
		return errors.New("data contract is required")
	}

	cType := ffi.GoStringToC(doc.documentType)
	defer C.free(unsafe.Pointer(cType))

	cSettings := convertPutSettings(settings)
	cPaymentInfo := convertTokenPaymentInfo(paymentInfo)

	err := ffi.PutDocumentToPlatform(doc.sdk.handle, doc.handle, cType, doc.dataContract.handle, cSettings, signedBy.handle, cPaymentInfo)
	
	return err
}

// PutAndWait publishes the document and waits for confirmation
func (doc *Document) PutAndWait(ctx context.Context, signedBy *Identity, settings *PutSettings, paymentInfo *TokenPaymentInfo) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("put"); err != nil {
		return err
	}

	if signedBy == nil || signedBy.handle == nil {
		return errors.New("signing identity is required")
	}

	if doc.dataContract == nil || doc.dataContract.handle == nil {
		return errors.New("data contract is required")
	}

	cType := ffi.GoStringToC(doc.documentType)
	defer C.free(unsafe.Pointer(cType))

	cSettings := convertPutSettings(settings)
	cPaymentInfo := convertTokenPaymentInfo(paymentInfo)

	err := ffi.PutDocumentToPlatformAndWait(doc.sdk.handle, doc.handle, cType, doc.dataContract.handle, cSettings, signedBy.handle, cPaymentInfo)
	
	return err
}

// Replace replaces the document on the platform
func (doc *Document) Replace(ctx context.Context, signedBy *Identity, settings *PutSettings, paymentInfo *TokenPaymentInfo) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("replace"); err != nil {
		return err
	}

	if signedBy == nil || signedBy.handle == nil {
		return errors.New("signing identity is required")
	}

	if doc.dataContract == nil || doc.dataContract.handle == nil {
		return errors.New("data contract is required")
	}

	cSettings := convertPutSettings(settings)
	cPaymentInfo := convertTokenPaymentInfo(paymentInfo)

	err := ffi.ReplaceDocumentOnPlatform(doc.sdk.handle, doc.handle, doc.dataContract.handle, cSettings, signedBy.handle, cPaymentInfo)
	
	return err
}

// Delete deletes the document from the platform
func (doc *Document) Delete(ctx context.Context, signedBy *Identity, settings *PutSettings) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("delete"); err != nil {
		return err
	}

	if signedBy == nil || signedBy.handle == nil {
		return errors.New("signing identity is required")
	}

	cSettings := convertPutSettings(settings)
	result := C.dash_sdk_document_delete(doc.sdk.handle, doc.handle, signedBy.handle, cSettings)
	_, err := ffi.HandleResult(result)
	
	return err
}

// Transfer transfers the document to another identity
func (doc *Document) Transfer(ctx context.Context, toIdentity IdentityID, signedBy *Identity, settings *PutSettings, paymentInfo *TokenPaymentInfo) (*DocumentTransferInfo, error) {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return nil, errors.New("SDK is closed")
	}

	if err := doc.requireHandle("transfer"); err != nil {
		return nil, err
	}

	if signedBy == nil || signedBy.handle == nil {
		return nil, errors.New("signing identity is required")
	}

	cToID := ffi.GoBytes32ToC(toIdentity)
	cSettings := convertPutSettings(settings)
	cPaymentInfo := convertTokenPaymentInfo(paymentInfo)

	result := C.dash_sdk_document_transfer_to_identity(doc.sdk.handle, doc.handle, cToID, signedBy.handle, cSettings, cPaymentInfo)
	data, err := ffi.HandleResult(result)
	if err != nil {
		return nil, fmt.Errorf("failed to transfer document: %w", err)
	}

	// Parse transfer info
	infoJSON := ffi.CStringToGoAndFree((*C.char)(data))
	var info DocumentTransferInfo
	if err := json.Unmarshal([]byte(infoJSON), &info); err != nil {
		return nil, fmt.Errorf("failed to parse transfer info: %w", err)
	}

	return &info, nil
}

// Purchase purchases the document
func (doc *Document) Purchase(ctx context.Context, purchaser *Identity, settings *PutSettings) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("purchase"); err != nil {
		return err
	}

	if purchaser == nil || purchaser.handle == nil {
		return errors.New("purchaser identity is required")
	}

	cSettings := convertPutSettings(settings)
	result := C.dash_sdk_document_purchase(doc.sdk.handle, doc.handle, purchaser.handle, cSettings)
	_, err := ffi.HandleResult(result)
	
	return err
}

// UpdatePrice updates the document price
func (doc *Document) UpdatePrice(ctx context.Context, newPrice uint64, signedBy *Identity, settings *PutSettings) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("update price"); err != nil {
		return err
	}

	if signedBy == nil || signedBy.handle == nil {
		return errors.New("signing identity is required")
	}

	cSettings := convertPutSettings(settings)
	result := C.dash_sdk_document_update_price_of_document(doc.sdk.handle, doc.handle, C.uint64_t(newPrice), signedBy.handle, cSettings)
	_, err := ffi.HandleResult(result)
	
	return err
}

// Destroy destroys the document on the platform
func (doc *Document) Destroy(ctx context.Context, settings *PutSettings) error {
	doc.sdk.mu.RLock()
	defer doc.sdk.mu.RUnlock()

	if doc.sdk.closed {
		return errors.New("SDK is closed")
	}

	if err := doc.requireHandle("destroy"); err != nil {
		return err
	}

	cSettings := convertPutSettings(settings)
	err := ffi.DestroyDocument(doc.sdk.handle, doc.handle, cSettings)
	
	return err
}

// Close releases the document handle
func (doc *Document) Close() error {
	if doc.handle != nil {
		ffi.DestroyDocumentHandleManual(doc.handle)
		doc.handle = nil
	}
	return nil
}

// DocumentTransferInfo contains information about a document transfer
type DocumentTransferInfo struct {
	TransactionID   string `json:"transactionId"`
	FromIdentityID  string `json:"fromIdentityId"`
	ToIdentityID    string `json:"toIdentityId"`
	DocumentID      string `json:"documentId"`
	TransferredAt   uint64 `json:"transferredAt"`
}

// QueryBuilder provides a fluent interface for building document queries
type QueryBuilder struct {
	query DocumentQuery
}

// NewQueryBuilder creates a new query builder
func NewQueryBuilder() *QueryBuilder {
	return &QueryBuilder{
		query: DocumentQuery{
			Where:   make(map[string]interface{}),
			OrderBy: make([]OrderClause, 0),
		},
	}
}

// Where adds a where clause
func (qb *QueryBuilder) Where(field string, value interface{}) *QueryBuilder {
	qb.query.Where[field] = value
	return qb
}

// WhereIn adds a where-in clause
func (qb *QueryBuilder) WhereIn(field string, values []interface{}) *QueryBuilder {
	qb.query.Where[field] = map[string]interface{}{
		"$in": values,
	}
	return qb
}

// WhereGT adds a greater-than clause
func (qb *QueryBuilder) WhereGT(field string, value interface{}) *QueryBuilder {
	qb.query.Where[field] = map[string]interface{}{
		"$gt": value,
	}
	return qb
}

// WhereLT adds a less-than clause
func (qb *QueryBuilder) WhereLT(field string, value interface{}) *QueryBuilder {
	qb.query.Where[field] = map[string]interface{}{
		"$lt": value,
	}
	return qb
}

// OrderBy adds an order by clause
func (qb *QueryBuilder) OrderBy(field string, ascending bool) *QueryBuilder {
	direction := "asc"
	if !ascending {
		direction = "desc"
	}
	qb.query.OrderBy = append(qb.query.OrderBy, OrderClause{
		Field:     field,
		Direction: direction,
	})
	return qb
}

// Limit sets the query limit
func (qb *QueryBuilder) Limit(limit int) *QueryBuilder {
	qb.query.Limit = limit
	return qb
}

// StartAt sets the start position
func (qb *QueryBuilder) StartAt(value interface{}) *QueryBuilder {
	qb.query.StartAt = value
	return qb
}

// StartAfter sets the start after position
func (qb *QueryBuilder) StartAfter(value interface{}) *QueryBuilder {
	qb.query.StartAfter = value
	return qb
}

// Build returns the built query
func (qb *QueryBuilder) Build() DocumentQuery {
	return qb.query
}