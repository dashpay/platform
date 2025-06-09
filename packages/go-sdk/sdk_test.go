package dash

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestVersion(t *testing.T) {
	version := Version()
	assert.NotEmpty(t, version, "SDK version should not be empty")
	t.Logf("SDK Version: %s", version)
}

func TestNewSDK(t *testing.T) {
	tests := []struct {
		name    string
		config  *Config
		wantErr bool
	}{
		{
			name:    "Default config",
			config:  DefaultConfig(),
			wantErr: false,
		},
		{
			name:    "Mainnet config",
			config:  ConfigMainnet(),
			wantErr: false,
		},
		{
			name:    "Testnet config",
			config:  ConfigTestnet(),
			wantErr: false,
		},
		{
			name:    "Devnet config",
			config:  ConfigDevnet(),
			wantErr: false,
		},
		{
			name:    "Local config",
			config:  ConfigLocal(),
			wantErr: false,
		},
		{
			name:    "Nil config uses default",
			config:  nil,
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sdk, err := NewSDK(tt.config)
			if tt.wantErr {
				assert.Error(t, err)
				return
			}
			require.NoError(t, err)
			require.NotNil(t, sdk)
			defer sdk.Close()

			// Verify sub-modules are initialized
			assert.NotNil(t, sdk.Identities())
			assert.NotNil(t, sdk.Contracts())
			assert.NotNil(t, sdk.Documents())
			assert.NotNil(t, sdk.Tokens())

			// Verify network matches config
			if tt.config != nil {
				network, err := sdk.GetNetwork()
				assert.NoError(t, err)
				assert.Equal(t, tt.config.Network, network)
			}
		})
	}
}

func TestNewMockSDK(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	require.NotNil(t, sdk)
	defer sdk.Close()

	// Verify mock SDK is properly initialized
	assert.NotNil(t, sdk.Identities())
	assert.NotNil(t, sdk.Contracts())
	assert.NotNil(t, sdk.Documents())
	assert.NotNil(t, sdk.Tokens())
}

func TestSDKClose(t *testing.T) {
	sdk, err := NewMockSDK()
	require.NoError(t, err)
	require.NotNil(t, sdk)

	// Close should work
	err = sdk.Close()
	assert.NoError(t, err)

	// Double close should be safe
	err = sdk.Close()
	assert.NoError(t, err)

	// Operations after close should fail
	_, err = sdk.GetNetwork()
	assert.Error(t, err)
}

func TestSDKGetNetwork(t *testing.T) {
	tests := []struct {
		name     string
		config   *Config
		expected Network
	}{
		{
			name:     "Mainnet",
			config:   ConfigMainnet(),
			expected: NetworkMainnet,
		},
		{
			name:     "Testnet",
			config:   ConfigTestnet(),
			expected: NetworkTestnet,
		},
		{
			name:     "Devnet",
			config:   ConfigDevnet(),
			expected: NetworkDevnet,
		},
		{
			name:     "Local",
			config:   ConfigLocal(),
			expected: NetworkLocal,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sdk, err := NewSDK(tt.config)
			require.NoError(t, err)
			defer sdk.Close()

			network, err := sdk.GetNetwork()
			require.NoError(t, err)
			assert.Equal(t, tt.expected, network)
			assert.Equal(t, tt.expected.String(), network.String())
		})
	}
}

func TestNetworkString(t *testing.T) {
	tests := []struct {
		network  Network
		expected string
	}{
		{NetworkMainnet, "mainnet"},
		{NetworkTestnet, "testnet"},
		{NetworkDevnet, "devnet"},
		{NetworkLocal, "local"},
		{Network(99), "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.expected, func(t *testing.T) {
			assert.Equal(t, tt.expected, tt.network.String())
		})
	}
}

func TestConfigPresets(t *testing.T) {
	t.Run("DefaultConfig", func(t *testing.T) {
		config := DefaultConfig()
		assert.Equal(t, NetworkTestnet, config.Network)
		assert.Equal(t, 3, config.MaxRetries)
		assert.True(t, config.BanFailedAddress)
	})

	t.Run("ConfigMainnet", func(t *testing.T) {
		config := ConfigMainnet()
		assert.Equal(t, NetworkMainnet, config.Network)
	})

	t.Run("ConfigTestnet", func(t *testing.T) {
		config := ConfigTestnet()
		assert.Equal(t, NetworkTestnet, config.Network)
		assert.NotEmpty(t, config.DAPIAddresses)
		assert.Contains(t, config.DAPIAddresses[0], "testnet")
	})

	t.Run("ConfigLocal", func(t *testing.T) {
		config := ConfigLocal()
		assert.Equal(t, NetworkLocal, config.Network)
		assert.NotEmpty(t, config.DAPIAddresses)
		assert.Contains(t, config.DAPIAddresses[0], "127.0.0.1")
		assert.NotEmpty(t, config.CoreRPCHost)
	})
}

func TestPutSettings(t *testing.T) {
	t.Run("DefaultPutSettings", func(t *testing.T) {
		settings := DefaultPutSettings()
		assert.NotNil(t, settings)
		assert.Equal(t, 3, settings.Retries)
		assert.True(t, settings.BanFailedAddress)
		assert.Greater(t, settings.ConnectTimeout.Seconds(), float64(0))
		assert.Greater(t, settings.RequestTimeout.Seconds(), float64(0))
	})
}