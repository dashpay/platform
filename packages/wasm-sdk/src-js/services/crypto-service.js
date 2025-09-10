/**
 * Crypto Service - Handles all cryptographic operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils, WasmOperationError } from '../error-handler.js';

export class CryptoService {
    /**
     * Create crypto service
     * @param {Object} wasmSdkWrapper - Reference to main WasmSDK wrapper instance
     * @param {Object} wasmSdkInstance - Raw WASM SDK instance
     * @param {Object} wasmModule - WASM module for operations
     * @param {Object} configManager - Configuration manager
     */
    constructor(wasmSdkWrapper, wasmSdkInstance, wasmModule, configManager) {
        this.wasmSdkWrapper = wasmSdkWrapper;
        this.wasmSdk = wasmSdkInstance;
        this.wasmModule = wasmModule;
        this.configManager = configManager;
    }

    /**
     * Generate a mnemonic phrase
     * @param {number} wordCount - Number of words (12, 15, 18, 21, or 24)
     * @returns {Promise<string>} Generated mnemonic phrase
     */
    async generateMnemonic(wordCount = 12) {
        ErrorUtils.validateRequired({ wordCount }, ['wordCount']);
        
        if (![12, 15, 18, 21, 24].includes(wordCount)) {
            throw new WasmOperationError(
                'Invalid word count. Must be 12, 15, 18, 21, or 24',
                'generate_mnemonic',
                { wordCount }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.generate_mnemonic(wordCount),
            'generate_mnemonic',
            { wordCount }
        );
    }

    /**
     * Validate a mnemonic phrase
     * @param {string} mnemonic - Mnemonic phrase to validate
     * @returns {Promise<boolean>} True if mnemonic is valid
     */
    async validateMnemonic(mnemonic) {
        ErrorUtils.validateRequired({ mnemonic }, ['mnemonic']);
        
        return this._executeOperation(
            () => this.wasmModule.validate_mnemonic(mnemonic),
            'validate_mnemonic',
            { mnemonic: '[SANITIZED]' }
        );
    }

    /**
     * Convert mnemonic to seed
     * @param {string} mnemonic - Mnemonic phrase
     * @param {string} passphrase - Optional passphrase
     * @returns {Promise<Uint8Array>} Generated seed
     */
    async mnemonicToSeed(mnemonic, passphrase = '') {
        ErrorUtils.validateRequired({ mnemonic }, ['mnemonic']);
        
        return this._executeOperation(
            () => this.wasmModule.mnemonic_to_seed(mnemonic, passphrase),
            'mnemonic_to_seed',
            { mnemonic: '[SANITIZED]', passphrase: passphrase ? '[SANITIZED]' : 'none' }
        );
    }

    /**
     * Derive key from seed with derivation path
     * @param {string} mnemonic - Mnemonic phrase
     * @param {string} passphrase - Optional passphrase
     * @param {string} path - Derivation path (e.g., "m/44'/5'/0'/0/0")
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<Object>} Object containing address, private_key_wif, and public_key
     */
    async deriveKeyFromSeedWithPath(mnemonic, passphrase = '', path, network = 'testnet') {
        ErrorUtils.validateRequired({ mnemonic, path, network }, ['mnemonic', 'path', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'derive_key_from_seed_with_path',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.derive_key_from_seed_with_path(mnemonic, passphrase, path, network),
            'derive_key_from_seed_with_path',
            { mnemonic: '[SANITIZED]', passphrase: passphrase ? '[SANITIZED]' : 'none', path, network }
        );
    }

    /**
     * Generate a key pair
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<Object>} Object containing private and public keys
     */
    async generateKeyPair(network = 'testnet') {
        ErrorUtils.validateRequired({ network }, ['network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'generate_key_pair',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.generate_key_pair(network),
            'generate_key_pair',
            { network }
        );
    }

    /**
     * Convert public key to address
     * @param {string} publicKey - Public key in hex format
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<string>} Generated address
     */
    async pubkeyToAddress(publicKey, network = 'testnet') {
        ErrorUtils.validateRequired({ publicKey, network }, ['publicKey', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'pubkey_to_address',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.pubkey_to_address(publicKey, network),
            'pubkey_to_address',
            { publicKey: '[SANITIZED]', network }
        );
    }

    /**
     * Validate an address
     * @param {string} address - Address to validate
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<boolean>} True if address is valid for the network
     */
    async validateAddress(address, network = 'testnet') {
        ErrorUtils.validateRequired({ address, network }, ['address', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'validate_address',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.validate_address(address, network),
            'validate_address',
            { address, network }
        );
    }

    /**
     * Sign a message
     * @param {string} message - Message to sign
     * @param {string} privateKey - Private key for signing
     * @returns {Promise<string>} Signature
     */
    async signMessage(message, privateKey) {
        ErrorUtils.validateRequired({ message, privateKey }, ['message', 'privateKey']);
        
        return this._executeOperation(
            () => this.wasmModule.sign_message(message, privateKey),
            'sign_message',
            { message, privateKey: '[SANITIZED]' }
        );
    }

    /**
     * Execute operation with proper error handling
     * @private
     * @param {Function} operation - Operation to execute
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context for errors
     * @returns {Promise<*>} Operation result
     */
    async _executeOperation(operation, operationName, context = {}) {
        return this.wasmSdkWrapper._executeOperation(operation, operationName, context);
    }
}