package dash

import (
	"encoding/hex"
	"encoding/json"
	"fmt"
)

// Network represents the Dash network type
type Network int

const (
	// NetworkMainnet represents the main Dash network
	NetworkMainnet Network = iota
	// NetworkTestnet represents the test network
	NetworkTestnet
	// NetworkDevnet represents the development network
	NetworkDevnet
	// NetworkLocal represents a local network
	NetworkLocal
)

// String returns the string representation of the network
func (n Network) String() string {
	switch n {
	case NetworkMainnet:
		return "mainnet"
	case NetworkTestnet:
		return "testnet"
	case NetworkDevnet:
		return "devnet"
	case NetworkLocal:
		return "local"
	default:
		return "unknown"
	}
}

// IdentityID represents a 32-byte identity identifier
type IdentityID [32]byte

// String returns the hex string representation of the identity ID
func (id IdentityID) String() string {
	return hex.EncodeToString(id[:])
}

// NewIdentityIDFromString creates an IdentityID from a hex string
func NewIdentityIDFromString(s string) (IdentityID, error) {
	var id IdentityID
	data, err := hex.DecodeString(s)
	if err != nil {
		return id, fmt.Errorf("invalid hex string: %w", err)
	}
	if len(data) != 32 {
		return id, fmt.Errorf("identity ID must be 32 bytes, got %d", len(data))
	}
	copy(id[:], data)
	return id, nil
}

// ContractID represents a 32-byte data contract identifier
type ContractID [32]byte

// String returns the hex string representation of the contract ID
func (id ContractID) String() string {
	return hex.EncodeToString(id[:])
}

// NewContractIDFromString creates a ContractID from a hex string
func NewContractIDFromString(s string) (ContractID, error) {
	var id ContractID
	data, err := hex.DecodeString(s)
	if err != nil {
		return id, fmt.Errorf("invalid hex string: %w", err)
	}
	if len(data) != 32 {
		return id, fmt.Errorf("contract ID must be 32 bytes, got %d", len(data))
	}
	copy(id[:], data)
	return id, nil
}

// DocumentID represents a 32-byte document identifier
type DocumentID [32]byte

// String returns the hex string representation of the document ID
func (id DocumentID) String() string {
	return hex.EncodeToString(id[:])
}

// NewDocumentIDFromString creates a DocumentID from a hex string
func NewDocumentIDFromString(s string) (DocumentID, error) {
	var id DocumentID
	data, err := hex.DecodeString(s)
	if err != nil {
		return id, fmt.Errorf("invalid hex string: %w", err)
	}
	if len(data) != 32 {
		return id, fmt.Errorf("document ID must be 32 bytes, got %d", len(data))
	}
	copy(id[:], data)
	return id, nil
}

// PublicKeyHash represents a 20-byte public key hash
type PublicKeyHash [20]byte

// String returns the hex string representation of the public key hash
func (h PublicKeyHash) String() string {
	return hex.EncodeToString(h[:])
}

// NewPublicKeyHashFromString creates a PublicKeyHash from a hex string
func NewPublicKeyHashFromString(s string) (PublicKeyHash, error) {
	var hash PublicKeyHash
	data, err := hex.DecodeString(s)
	if err != nil {
		return hash, fmt.Errorf("invalid hex string: %w", err)
	}
	if len(data) != 20 {
		return hash, fmt.Errorf("public key hash must be 20 bytes, got %d", len(data))
	}
	copy(hash[:], data)
	return hash, nil
}

// IdentityInfo contains information about an identity
type IdentityInfo struct {
	ID             string            `json:"id"`
	Balance        uint64            `json:"balance"`
	PublicKeys     []PublicKeyInfo   `json:"publicKeys"`
	ContractBounds map[string]uint64 `json:"contractBounds,omitempty"`
}

// PublicKeyInfo contains information about a public key
type PublicKeyInfo struct {
	ID                 uint64 `json:"id"`
	Type               uint8  `json:"type"`
	Purpose            uint8  `json:"purpose"`
	SecurityLevel      uint8  `json:"securityLevel"`
	Data               []byte `json:"data"`
	ReadOnly           bool   `json:"readOnly"`
	DisabledAt         uint64 `json:"disabledAt,omitempty"`
	ContractBounds     map[string]map[string]interface{} `json:"contractBounds,omitempty"`
}

// DocumentInfo contains information about a document
type DocumentInfo struct {
	ID             string                 `json:"id"`
	OwnerID        string                 `json:"ownerId"`
	DataContractID string                 `json:"dataContractId"`
	DocumentType   string                 `json:"documentType"`
	Revision       uint64                 `json:"revision"`
	CreatedAt      uint64                 `json:"createdAt"`
	UpdatedAt      uint64                 `json:"updatedAt"`
	Data           map[string]interface{} `json:"data"`
}

// DataContractInfo contains information about a data contract
type DataContractInfo struct {
	ID            string                            `json:"id"`
	OwnerID       string                            `json:"ownerId"`
	Version       uint64                            `json:"version"`
	Schema        map[string]json.RawMessage        `json:"schema"`
	DocumentTypes map[string]DocumentTypeDefinition `json:"documentTypes"`
}

// DocumentTypeDefinition defines a document type in a data contract
type DocumentTypeDefinition struct {
	Type                 string                    `json:"type"`
	Properties           map[string]PropertySchema `json:"properties"`
	Required             []string                  `json:"required,omitempty"`
	AdditionalProperties bool                      `json:"additionalProperties"`
	Indices              []Index                   `json:"indices,omitempty"`
}

// PropertySchema defines a property in a document type
type PropertySchema struct {
	Type        interface{} `json:"type"` // string or []string for multiple types
	Format      string      `json:"format,omitempty"`
	Minimum     *float64    `json:"minimum,omitempty"`
	Maximum     *float64    `json:"maximum,omitempty"`
	MaxLength   *int        `json:"maxLength,omitempty"`
	MinLength   *int        `json:"minLength,omitempty"`
	Pattern     string      `json:"pattern,omitempty"`
	Items       *PropertySchema `json:"items,omitempty"`
	Properties  map[string]PropertySchema `json:"properties,omitempty"`
	Required    []string    `json:"required,omitempty"`
	Description string      `json:"description,omitempty"`
}

// Index defines an index for a document type
type Index struct {
	Name       string            `json:"name"`
	Properties []IndexProperty   `json:"properties"`
	Unique     bool              `json:"unique,omitempty"`
	Sparse     bool              `json:"sparse,omitempty"`
}

// IndexProperty defines a property in an index
type IndexProperty struct {
	Name string `json:"$propertyName"`
	Asc  string `json:"$direction,omitempty"` // "asc" or "desc"
}

// DocumentQuery represents a query for documents
type DocumentQuery struct {
	Where    map[string]interface{} `json:"where,omitempty"`
	OrderBy  []OrderClause          `json:"orderBy,omitempty"`
	Limit    int                    `json:"limit,omitempty"`
	StartAt  interface{}            `json:"startAt,omitempty"`
	StartAfter interface{}          `json:"startAfter,omitempty"`
}

// OrderClause represents an order by clause
type OrderClause struct {
	Field     string `json:"field"`
	Direction string `json:"direction"` // "asc" or "desc"
}

// TokenInfo contains information about a token
type TokenInfo struct {
	ContractID         string `json:"contractId"`
	Position           uint16 `json:"position"`
	TotalSupply        uint64 `json:"totalSupply"`
	RemainingSupply    uint64 `json:"remainingSupply"`
	CirculatingSupply  uint64 `json:"circulatingSupply"`
	MaxSupply          uint64 `json:"maxSupply"`
	Decimals           uint8  `json:"decimals"`
}

// GasFeesPaidBy specifies who pays gas fees
type GasFeesPaidBy int

const (
	// GasFeesDocumentOwner means the document owner pays fees
	GasFeesDocumentOwner GasFeesPaidBy = iota
	// GasFeesContractOwner means the contract owner pays fees
	GasFeesContractOwner
	// GasFeesPreferContractOwner prefers contract owner but falls back to document owner
	GasFeesPreferContractOwner
)

// TokenPaymentInfo contains payment information for token operations
type TokenPaymentInfo struct {
	PaymentTokenContractID *ContractID   `json:"paymentTokenContractId,omitempty"`
	TokenContractPosition  uint16        `json:"tokenContractPosition"`
	MinimumTokenCost       uint64        `json:"minimumTokenCost,omitempty"`
	MaximumTokenCost       uint64        `json:"maximumTokenCost,omitempty"`
	GasFeesPaidBy          GasFeesPaidBy `json:"gasFeesPaidBy"`
}