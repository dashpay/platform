package dash

import (
	"time"
)

// Config represents SDK configuration
type Config struct {
	// Network type (mainnet, testnet, devnet, local)
	Network Network

	// DAPI addresses (optional, uses default seeds if empty)
	DAPIAddresses []string

	// Retry configuration
	MaxRetries         int
	RetryDelay         time.Duration
	BanFailedAddress   bool

	// Timeout configuration
	ConnectTimeout     time.Duration
	RequestTimeout     time.Duration
	WaitTimeout        time.Duration

	// Identity configuration
	IdentityNonceStaleTime time.Duration

	// Fee configuration
	UserFeeIncrease uint16 // Additional percentage of processing fee

	// Debug options
	AllowSigningWithAnySecurityLevel bool
	AllowSigningWithAnyPurpose       bool

	// Core configuration for asset locks
	CoreRPCHost     string
	CoreRPCUsername string
	CoreRPCPassword string
}

// DefaultConfig returns default configuration
func DefaultConfig() *Config {
	return &Config{
		Network:                NetworkTestnet,
		MaxRetries:             3,
		RetryDelay:             1 * time.Second,
		BanFailedAddress:       true,
		ConnectTimeout:         10 * time.Second,
		RequestTimeout:         60 * time.Second,
		WaitTimeout:            30 * time.Second,
		IdentityNonceStaleTime: 5 * time.Minute,
		UserFeeIncrease:        0,
	}
}

// ConfigMainnet returns mainnet configuration
func ConfigMainnet() *Config {
	config := DefaultConfig()
	config.Network = NetworkMainnet
	config.DAPIAddresses = []string{
		// Add mainnet seed nodes when available
	}
	return config
}

// ConfigTestnet returns testnet configuration
func ConfigTestnet() *Config {
	config := DefaultConfig()
	config.Network = NetworkTestnet
	config.DAPIAddresses = []string{
		"testnet-seed-1.dashevo.org:443",
		"testnet-seed-2.dashevo.org:443",
		"testnet-seed-3.dashevo.org:443",
		"testnet-seed-4.dashevo.org:443",
		"testnet-seed-5.dashevo.org:443",
	}
	return config
}

// ConfigDevnet returns devnet configuration
func ConfigDevnet() *Config {
	config := DefaultConfig()
	config.Network = NetworkDevnet
	config.DAPIAddresses = []string{
		// Add devnet addresses when needed
	}
	return config
}

// ConfigLocal returns local network configuration
func ConfigLocal() *Config {
	config := DefaultConfig()
	config.Network = NetworkLocal
	config.DAPIAddresses = []string{
		"127.0.0.1:3000",
		"127.0.0.1:3001",
		"127.0.0.1:3002",
	}
	config.CoreRPCHost = "127.0.0.1:19998"
	config.CoreRPCUsername = "dashrpc"
	config.CoreRPCPassword = "password"
	return config
}

// PutSettings represents settings for platform operations
type PutSettings struct {
	// Timeout for establishing a connection
	ConnectTimeout time.Duration

	// Timeout for single request
	RequestTimeout time.Duration

	// Number of retries in case of failed requests
	Retries int

	// Ban DAPI address if node not responded or responded with error
	BanFailedAddress bool

	// Identity nonce stale time
	IdentityNonceStaleTime time.Duration

	// User fee increase (additional percentage of processing fee)
	UserFeeIncrease uint16

	// Enable signing with any security level (for debugging)
	AllowSigningWithAnySecurityLevel bool

	// Enable signing with any purpose (for debugging)
	AllowSigningWithAnyPurpose bool

	// Wait timeout for operations
	WaitTimeout time.Duration
}

// DefaultPutSettings returns default put settings
func DefaultPutSettings() *PutSettings {
	return &PutSettings{
		ConnectTimeout:         10 * time.Second,
		RequestTimeout:         60 * time.Second,
		Retries:                3,
		BanFailedAddress:       true,
		IdentityNonceStaleTime: 5 * time.Minute,
		UserFeeIncrease:        0,
		WaitTimeout:            30 * time.Second,
	}
}

// StateTransitionCreationOptions represents options for state transition creation
type StateTransitionCreationOptions struct {
	// Allow signing with any security level (for debugging)
	AllowSigningWithAnySecurityLevel bool

	// Allow signing with any purpose (for debugging)
	AllowSigningWithAnyPurpose bool

	// Feature versions (0 means use default)
	BatchFeatureVersion  uint16
	MethodFeatureVersion uint16
	BaseFeatureVersion   uint16
}