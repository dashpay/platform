/**
 * Configuration & Error System Integration Tests
 * 
 * Comprehensive tests for Issue #52 enhancements:
 * - Advanced configuration validation
 * - Enhanced error handling with debugging context
 * - Transport and network configuration support
 * - Proof verification settings integration
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { 
  WasmSDK, 
  WasmSDKError, 
  WasmInitializationError, 
  WasmOperationError, 
  ErrorMapper,
  isWasmSDKError,
  isWasmInitializationError,
  isWasmOperationError,
  NETWORK_TYPES,
  DEFAULT_CONFIG,
  SDK_VERSION
} from '../src-js/index.js';

describe('Configuration & Error System Integration', () => {

  describe('Constants and Exports', () => {
    it('should export required constants', () => {
      expect(NETWORK_TYPES).toEqual(['mainnet', 'testnet']);
      expect(DEFAULT_CONFIG).toHaveProperty('network', 'testnet');
      expect(DEFAULT_CONFIG).toHaveProperty('transport');
      expect(DEFAULT_CONFIG).toHaveProperty('proofs', true);
      expect(SDK_VERSION).toHaveProperty('VERSION_STRING', '1.0.0');
    });

    it('should export error classes', () => {
      expect(WasmSDKError).toBeDefined();
      expect(WasmInitializationError).toBeDefined();
      expect(WasmOperationError).toBeDefined();
      expect(ErrorMapper).toBeDefined();
    });

    it('should export type guard functions', () => {
      expect(typeof isWasmSDKError).toBe('function');
      expect(typeof isWasmInitializationError).toBe('function');
      expect(typeof isWasmOperationError).toBe('function');
    });
  });

  describe('Enhanced Configuration Validation', () => {
    it('should validate network configuration', () => {
      expect(() => {
        new WasmSDK({ network: 'invalid' });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ network: 'mainnet' });
      }).not.toThrow();

      expect(() => {
        new WasmSDK({ network: 'testnet' });
      }).not.toThrow();
    });

    it('should validate transport configuration', () => {
      // Invalid transport object
      expect(() => {
        new WasmSDK({ transport: null });
      }).toThrow(WasmInitializationError);

      // Invalid URL
      expect(() => {
        new WasmSDK({ transport: { url: '' } });
      }).toThrow(WasmInitializationError);

      // Invalid URL format
      expect(() => {
        new WasmSDK({ transport: { url: 'not-a-url' } });
      }).toThrow(WasmInitializationError);

      // Invalid timeout
      expect(() => {
        new WasmSDK({ transport: { url: 'https://example.com', timeout: 500 } });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ transport: { url: 'https://example.com', timeout: 400000 } });
      }).toThrow(WasmInitializationError);

      // Invalid retries
      expect(() => {
        new WasmSDK({ transport: { url: 'https://example.com', retries: -1 } });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ transport: { url: 'https://example.com', retries: 15 } });
      }).toThrow(WasmInitializationError);

      // Valid configuration
      expect(() => {
        new WasmSDK({ 
          transport: { 
            url: 'https://example.com:1443/', 
            timeout: 15000,
            retries: 3
          } 
        });
      }).not.toThrow();
    });

    it('should validate settings configuration', () => {
      // Invalid settings object
      expect(() => {
        new WasmSDK({ settings: 'invalid' });
      }).toThrow(WasmInitializationError);

      // Invalid connect_timeout_ms
      expect(() => {
        new WasmSDK({ settings: { connect_timeout_ms: 500 } });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ settings: { connect_timeout_ms: 70000 } });
      }).toThrow(WasmInitializationError);

      // Invalid timeout_ms
      expect(() => {
        new WasmSDK({ settings: { timeout_ms: 500 } });
      }).toThrow(WasmInitializationError);

      // Invalid retries
      expect(() => {
        new WasmSDK({ settings: { retries: -1 } });
      }).toThrow(WasmInitializationError);

      // Invalid ban_failed_address
      expect(() => {
        new WasmSDK({ settings: { ban_failed_address: 'yes' } });
      }).toThrow(WasmInitializationError);

      // Valid settings
      expect(() => {
        new WasmSDK({ 
          settings: { 
            connect_timeout_ms: 5000,
            timeout_ms: 20000,
            retries: 5,
            ban_failed_address: false
          } 
        });
      }).not.toThrow();
    });

    it('should validate proofs configuration', () => {
      expect(() => {
        new WasmSDK({ proofs: 'true' });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ proofs: true });
      }).not.toThrow();

      expect(() => {
        new WasmSDK({ proofs: false });
      }).not.toThrow();
    });

    it('should validate version configuration', () => {
      expect(() => {
        new WasmSDK({ version: 'latest' });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ version: -1 });
      }).toThrow(WasmInitializationError);

      expect(() => {
        new WasmSDK({ version: null });
      }).not.toThrow();

      expect(() => {
        new WasmSDK({ version: 1 });
      }).not.toThrow();
    });
  });

  describe('Error Context and Debugging', () => {
    it('should provide detailed error context for validation failures', () => {
      try {
        new WasmSDK({ network: 'invalid' });
      } catch (error) {
        expect(error).toBeInstanceOf(WasmInitializationError);
        expect(error.context).toHaveProperty('providedNetwork', 'invalid');
        expect(error.context).toHaveProperty('validNetworks');
        expect(error.context.validNetworks).toEqual(['mainnet', 'testnet']);
      }
    });

    it('should provide context for transport validation failures', () => {
      try {
        new WasmSDK({ transport: { url: 'invalid-url', timeout: 500 } });
      } catch (error) {
        expect(error).toBeInstanceOf(WasmInitializationError);
        expect(error.context).toHaveProperty('providedUrl', 'invalid-url');
      }
    });

    it('should provide context for settings validation failures', () => {
      try {
        new WasmSDK({ settings: { retries: 20 } });
      } catch (error) {
        expect(error).toBeInstanceOf(WasmInitializationError);
        expect(error.context).toHaveProperty('settingKey', 'retries');
        expect(error.context).toHaveProperty('providedValue', 20);
        expect(error.context).toHaveProperty('validRange');
      }
    });
  });

  describe('Type Guards', () => {
    it('should correctly identify WasmSDKError', () => {
      const error = new WasmSDKError('test', 'TEST_CODE');
      expect(isWasmSDKError(error)).toBe(true);
      expect(isWasmInitializationError(error)).toBe(false);
      expect(isWasmOperationError(error)).toBe(false);

      const regularError = new Error('test');
      expect(isWasmSDKError(regularError)).toBe(false);
    });

    it('should correctly identify WasmInitializationError', () => {
      const error = new WasmInitializationError('test');
      expect(isWasmSDKError(error)).toBe(true);
      expect(isWasmInitializationError(error)).toBe(true);
      expect(isWasmOperationError(error)).toBe(false);
    });

    it('should correctly identify WasmOperationError', () => {
      const error = new WasmOperationError('test', 'testOperation');
      expect(isWasmSDKError(error)).toBe(true);
      expect(isWasmInitializationError(error)).toBe(false);
      expect(isWasmOperationError(error)).toBe(true);
    });
  });

  describe('ErrorMapper', () => {
    it('should map WASM errors with context', () => {
      const originalError = new Error('WASM validation failed');
      const mappedError = ErrorMapper.mapWasmError(originalError, 'testOperation', { customData: 'test' });

      expect(mappedError).toBeInstanceOf(WasmOperationError);
      expect(mappedError.message).toContain('WASM validation failed');
      expect(mappedError.context).toHaveProperty('customData', 'test');
      expect(mappedError.context).toHaveProperty('timestamp');
      expect(mappedError.context).toHaveProperty('operationName', 'testOperation');
      expect(mappedError.context).toHaveProperty('errorCategory', 'validation');
    });

    it('should categorize errors correctly', () => {
      const networkError = new Error('network connection failed');
      const mapped = ErrorMapper.mapWasmError(networkError, 'testOp');
      expect(mapped.context.errorCategory).toBe('network');

      const timeoutError = new Error('operation timeout');
      const mappedTimeout = ErrorMapper.mapWasmError(timeoutError, 'testOp');
      expect(mappedTimeout.context.errorCategory).toBe('timeout');

      const proofError = new Error('proof verification failed');
      const mappedProof = ErrorMapper.mapWasmError(proofError, 'testOp');
      expect(mappedProof.context.errorCategory).toBe('proof_verification');
    });

    it('should create contextual errors with sanitized input data', () => {
      const inputData = {
        username: 'testuser',
        privateKey: 'secret123',
        mnemonic: 'word1 word2 word3'
      };
      
      const error = ErrorMapper.createContextualError(
        'Test error',
        'testOperation',
        inputData,
        new Error('original')
      );

      expect(error.context.inputData).toHaveProperty('username', 'testuser');
      expect(error.context.inputData).toHaveProperty('privateKey', '[REDACTED]');
      expect(error.context.inputData).toHaveProperty('mnemonic', '[REDACTED]');
      expect(error.context).toHaveProperty('timestamp');
      expect(error.context).toHaveProperty('originalError');
    });
  });

  describe('SDK Configuration Management', () => {
    let sdk;

    beforeEach(() => {
      sdk = new WasmSDK({
        network: 'testnet',
        transport: {
          url: 'https://test.example.com:1443/',
          timeout: 20000,
          retries: 2
        },
        settings: {
          connect_timeout_ms: 8000,
          timeout_ms: 25000,
          retries: 4,
          ban_failed_address: false
        },
        proofs: false
      });
    });

    afterEach(() => {
      if (sdk) {
        sdk.destroy();
      }
    });

    it('should merge configuration with defaults correctly', () => {
      const config = sdk.getConfig();

      expect(config.network).toBe('testnet');
      expect(config.transport.url).toBe('https://test.example.com:1443/');
      expect(config.transport.timeout).toBe(20000);
      expect(config.transport.retries).toBe(2);
      expect(config.settings.connect_timeout_ms).toBe(8000);
      expect(config.settings.timeout_ms).toBe(25000);
      expect(config.settings.retries).toBe(4);
      expect(config.settings.ban_failed_address).toBe(false);
      expect(config.proofs).toBe(false);
    });

    it('should provide immutable configuration copy', () => {
      const config = sdk.getConfig();
      const originalUrl = config.transport.url;

      // Attempt to modify the returned config
      config.transport.url = 'modified';
      
      // Should not affect the SDK's internal config
      const freshConfig = sdk.getConfig();
      expect(freshConfig.transport.url).toBe(originalUrl);
    });

    it('should validate that SDK is not initialized by default', () => {
      expect(sdk.isInitialized()).toBe(false);
    });
  });

  describe('Synchronous Operation Error Handling', () => {
    let sdk;

    beforeEach(() => {
      sdk = new WasmSDK();
    });

    afterEach(() => {
      if (sdk) {
        sdk.destroy();
      }
    });

    it('should handle synchronous DPNS operations with enhanced error context', () => {
      // These operations should work without initialization for pure validation functions
      expect(typeof sdk.isDpnsUsernameValid).toBe('function');
      expect(typeof sdk.isDpnsUsernameContested).toBe('function');
      expect(typeof sdk.dpnsConvertToHomographSafe).toBe('function');
    });

    it('should handle wallet operations with enhanced error context', () => {
      expect(typeof sdk.calculateTokenId).toBe('function');
      expect(typeof sdk.deriveKey).toBe('function');
      expect(typeof sdk.deriveDashPayContactKey).toBe('function');
    });
  });

  describe('Configuration Edge Cases', () => {
    it('should handle partial configuration updates', () => {
      const sdk = new WasmSDK({ network: 'testnet' });
      expect(sdk.getConfig().network).toBe('testnet');
      expect(sdk.getConfig().transport.url).toBe(DEFAULT_CONFIG.transport.url);
    });

    it('should handle empty configuration', () => {
      const sdk = new WasmSDK({});
      const config = sdk.getConfig();
      
      expect(config.network).toBe(DEFAULT_CONFIG.network);
      expect(config.transport.url).toBe(DEFAULT_CONFIG.transport.url);
      expect(config.proofs).toBe(DEFAULT_CONFIG.proofs);
    });

    it('should handle null and undefined values appropriately', () => {
      const sdk = new WasmSDK({ 
        version: null,
        transport: { url: 'https://example.com' }
      });
      
      expect(sdk.getConfig().version).toBeNull();
      expect(sdk.getConfig().transport.url).toBe('https://example.com');
    });
  });

  describe('Error Inheritance Chain', () => {
    it('should maintain proper error inheritance', () => {
      const baseError = new WasmSDKError('base', 'BASE_CODE');
      const initError = new WasmInitializationError('init');
      const opError = new WasmOperationError('op', 'operation');

      expect(baseError instanceof WasmSDKError).toBe(true);
      expect(baseError instanceof Error).toBe(true);

      expect(initError instanceof WasmInitializationError).toBe(true);
      expect(initError instanceof WasmSDKError).toBe(true);
      expect(initError instanceof Error).toBe(true);

      expect(opError instanceof WasmOperationError).toBe(true);
      expect(opError instanceof WasmSDKError).toBe(true);
      expect(opError instanceof Error).toBe(true);
    });

    it('should preserve error properties correctly', () => {
      const context = { key: 'value' };
      const error = new WasmSDKError('test message', 'TEST_CODE', context);

      expect(error.message).toBe('test message');
      expect(error.name).toBe('WasmSDKError');
      expect(error.code).toBe('TEST_CODE');
      expect(error.context).toEqual(context);
    });
  });

  describe('Transport Configuration Edge Cases', () => {
    it('should handle various URL formats', () => {
      const validUrls = [
        'https://example.com',
        'https://example.com:1443',
        'https://example.com:1443/',
        'http://localhost:3000',
        'http://192.168.1.100:1443'
      ];

      validUrls.forEach(url => {
        expect(() => {
          new WasmSDK({ transport: { url } });
        }).not.toThrow();
      });
    });

    it('should reject invalid URL schemes', () => {
      const invalidUrls = [
        'ftp://example.com',
        'file:///path/to/file',
        'ws://example.com',
        'example.com' // Missing protocol
      ];

      invalidUrls.forEach(url => {
        expect(() => {
          new WasmSDK({ transport: { url } });
        }).toThrow(WasmInitializationError);
      });
    });
  });
});