package dash

import (
	"errors"
	"fmt"
)

// Common errors
var (
	// ErrNilSDK is returned when SDK handle is nil
	ErrNilSDK = errors.New("SDK handle is nil")

	// ErrNilIdentity is returned when identity handle is nil
	ErrNilIdentity = errors.New("identity handle is nil")

	// ErrNilDocument is returned when document handle is nil
	ErrNilDocument = errors.New("document handle is nil")

	// ErrNilDataContract is returned when data contract handle is nil
	ErrNilDataContract = errors.New("data contract handle is nil")

	// ErrInvalidParameter is returned when an invalid parameter is provided
	ErrInvalidParameter = errors.New("invalid parameter")

	// ErrNotFound is returned when a requested resource is not found
	ErrNotFound = errors.New("resource not found")

	// ErrTimeout is returned when an operation times out
	ErrTimeout = errors.New("operation timed out")

	// ErrNetworkError is returned when a network error occurs
	ErrNetworkError = errors.New("network error")

	// ErrSerializationError is returned when serialization fails
	ErrSerializationError = errors.New("serialization error")

	// ErrCryptoError is returned when a cryptographic operation fails
	ErrCryptoError = errors.New("cryptographic error")

	// ErrProtocolError is returned when a protocol error occurs
	ErrProtocolError = errors.New("protocol error")

	// ErrNotImplemented is returned when a feature is not implemented
	ErrNotImplemented = errors.New("feature not implemented")

	// ErrInternalError is returned when an internal error occurs
	ErrInternalError = errors.New("internal error")
)

// SDKError represents an error from the SDK
type SDKError struct {
	Code    ErrorCode
	Message string
}

// Error implements the error interface
func (e *SDKError) Error() string {
	return fmt.Sprintf("SDK error %d: %s", e.Code, e.Message)
}

// Is implements error matching
func (e *SDKError) Is(target error) bool {
	if target == nil {
		return false
	}

	switch e.Code {
	case ErrorCodeInvalidParameter:
		return target == ErrInvalidParameter
	case ErrorCodeNotFound:
		return target == ErrNotFound
	case ErrorCodeTimeout:
		return target == ErrTimeout
	case ErrorCodeNetworkError:
		return target == ErrNetworkError
	case ErrorCodeSerializationError:
		return target == ErrSerializationError
	case ErrorCodeCryptoError:
		return target == ErrCryptoError
	case ErrorCodeProtocolError:
		return target == ErrProtocolError
	case ErrorCodeNotImplemented:
		return target == ErrNotImplemented
	case ErrorCodeInternalError:
		return target == ErrInternalError
	}

	if targetErr, ok := target.(*SDKError); ok {
		return e.Code == targetErr.Code
	}

	return false
}

// ErrorCode represents SDK error codes
type ErrorCode int

const (
	// ErrorCodeSuccess indicates success (no error)
	ErrorCodeSuccess ErrorCode = 0

	// ErrorCodeInvalidParameter indicates an invalid parameter
	ErrorCodeInvalidParameter ErrorCode = 1

	// ErrorCodeInvalidState indicates SDK is in invalid state
	ErrorCodeInvalidState ErrorCode = 2

	// ErrorCodeNetworkError indicates a network error
	ErrorCodeNetworkError ErrorCode = 3

	// ErrorCodeSerializationError indicates serialization failed
	ErrorCodeSerializationError ErrorCode = 4

	// ErrorCodeProtocolError indicates a protocol error
	ErrorCodeProtocolError ErrorCode = 5

	// ErrorCodeCryptoError indicates a cryptographic error
	ErrorCodeCryptoError ErrorCode = 6

	// ErrorCodeNotFound indicates resource not found
	ErrorCodeNotFound ErrorCode = 7

	// ErrorCodeTimeout indicates operation timed out
	ErrorCodeTimeout ErrorCode = 8

	// ErrorCodeNotImplemented indicates feature not implemented
	ErrorCodeNotImplemented ErrorCode = 9

	// ErrorCodeInternalError indicates an internal error
	ErrorCodeInternalError ErrorCode = 99
)

// String returns the string representation of the error code
func (c ErrorCode) String() string {
	switch c {
	case ErrorCodeSuccess:
		return "Success"
	case ErrorCodeInvalidParameter:
		return "InvalidParameter"
	case ErrorCodeInvalidState:
		return "InvalidState"
	case ErrorCodeNetworkError:
		return "NetworkError"
	case ErrorCodeSerializationError:
		return "SerializationError"
	case ErrorCodeProtocolError:
		return "ProtocolError"
	case ErrorCodeCryptoError:
		return "CryptoError"
	case ErrorCodeNotFound:
		return "NotFound"
	case ErrorCodeTimeout:
		return "Timeout"
	case ErrorCodeNotImplemented:
		return "NotImplemented"
	case ErrorCodeInternalError:
		return "InternalError"
	default:
		return fmt.Sprintf("Unknown(%d)", c)
	}
}

// NewSDKError creates a new SDK error
func NewSDKError(code ErrorCode, message string) *SDKError {
	return &SDKError{
		Code:    code,
		Message: message,
	}
}