import { DAPIClient } from '@dashevo/dapi-client';

// Import type definitions
/// <reference path="../types/wasm-sdk.d.ts" />

// Type definitions for wasm-sdk module
interface WasmSdkModule {
  default: () => Promise<void>;
  initSync?: (buffer: Buffer) => void;
  WasmSdk: any; // Will be properly typed when wasm-sdk is available
  WasmSdkBuilder: any; // Builder class for creating SDK instances
}

export class WasmPlatformAdapter {
  private wasmSdk?: WasmSdkModule;
  private sdkInstance?: any;
  private initialized = false;
  private dapiClient: DAPIClient;
  private network: string;
  private proofs: boolean;
  private platform?: any; // Reference to Platform instance
  
  // Cache for frequently accessed data
  private cache: Map<string, { data: any; timestamp: number }> = new Map();
  private cacheTTL = 60000; // 60 seconds default TTL

  constructor(dapiClient: DAPIClient, network: string, proofs: boolean = true) {
    console.log('WasmPlatformAdapter constructor:', {
      dapiClient: dapiClient ? 'present' : 'missing',
      hasDapiAddresses: dapiClient ? dapiClient.dapiAddresses : 'N/A',
      network,
      proofs
    });
    this.dapiClient = dapiClient;
    this.network = network;
    this.proofs = proofs;
  }

  /**
   * Set the platform instance for response conversion
   */
  setPlatform(platform: any): void {
    this.platform = platform;
  }

  /**
   * Initialize the WASM SDK module dynamically
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Use loader for proper ES module handling in Node.js
      // @ts-ignore - Using JS loader
      const { loadWasmSdk } = require('./wasm-sdk-loader');
      const wasmModule = await loadWasmSdk() as WasmSdkModule;
      
      // Initialize WASM module
      console.log('WasmPlatformAdapter: Initializing WASM module...');
      try {
        // In Node.js environments, we need to handle the WASM file path
        if (typeof window === 'undefined' && typeof global !== 'undefined') {
          // Node.js environment - use initSync with buffer
          const fs = require('fs');
          const path = require('path');
          const wasmPath = path.join(__dirname, '..', '..', '..', '..', '..', '..', '..', '.yarn', 'cache', '@dashevo-wasm-sdk-file-7f6fe61b82-1ccf5cd50c.zip', 'node_modules', '@dashevo', 'wasm-sdk', 'wasm_sdk_bg.wasm');
          console.log('WasmPlatformAdapter: Loading WASM from:', wasmPath);
          const wasmBuffer = fs.readFileSync(wasmPath);
          
          // Use initSync for synchronous initialization in Node.js
          if (wasmModule.initSync) {
            wasmModule.initSync(wasmBuffer);
          } else {
            // Fallback to default init
            await wasmModule.default();
          }
        } else {
          // Browser environment
          await wasmModule.default();
        }
      } catch (initError) {
        console.error('WasmPlatformAdapter: WASM module initialization error:', initError);
        throw new Error(`Failed to initialize WASM module: ${initError.message}`);
      }
      
      this.wasmSdk = wasmModule;
      this.initialized = true;
      console.log('WasmPlatformAdapter: WASM module initialized successfully');
      console.log('WasmPlatformAdapter: Available exports:', Object.keys(wasmModule));
    } catch (error) {
      throw new Error(`Failed to initialize wasm-sdk: ${error.message}`);
    }
  }

  /**
   * Get or create SDK instance
   */
  async getSdk(): Promise<any> {
    if (!this.initialized) {
      await this.initialize();
    }

    if (!this.sdkInstance) {
      try {
        // Use WasmSdkBuilder to create SDK instance
        let builder;
        if (this.network === 'mainnet') {
          builder = this.wasmSdk!.WasmSdkBuilder.new_mainnet();
        } else if (this.network === 'testnet') {
          builder = this.wasmSdk!.WasmSdkBuilder.new_testnet();
        } else {
          // For regtest/local, use testnet configuration
          // TODO: For local/regtest networks, we need to configure custom URL
          console.log('WasmPlatformAdapter: Using testnet builder for regtest network');
          builder = this.wasmSdk!.WasmSdkBuilder.new_testnet();
        }
        
        // Build the SDK instance
        this.sdkInstance = builder.build();
        console.log('WasmPlatformAdapter: SDK instance created successfully');
      } catch (error) {
        console.error('WasmPlatformAdapter: Failed to create SDK instance:', error);
        throw error;
      }
    }

    return this.sdkInstance;
  }

  /**
   * Convert js-dash-sdk asset lock to wasm-sdk format
   */
  convertAssetLockProof(assetLock: any): string {
    // Convert asset lock from js-dash-sdk format to wasm-sdk hex format
    const proofObject = {
      type: assetLock.type || 0,
      transaction: assetLock.transaction.toString('hex'),
      outputIndex: assetLock.outputIndex,
      instantLock: assetLock.instantLock ? assetLock.instantLock.toString('hex') : undefined,
    };

    return JSON.stringify(proofObject);
  }

  /**
   * Convert private key to WIF format
   */
  convertPrivateKeyToWIF(privateKey: any): string {
    // If already WIF, return as is
    if (typeof privateKey === 'string' && (privateKey.startsWith('X') || privateKey.startsWith('7'))) {
      return privateKey;
    }

    // Convert PrivateKey object to WIF
    if (privateKey && typeof privateKey.toWIF === 'function') {
      return privateKey.toWIF();
    }

    throw new Error('Invalid private key format');
  }

  /**
   * Convert wasm-sdk response to js-dash-sdk format
   */
  convertResponse(wasmResponse: any, responseType: string): any {
    // This will be extended as we implement more methods
    switch (responseType) {
      case 'identity':
        return this.convertIdentityResponse(wasmResponse);
      case 'document':
        return this.convertDocumentResponse(wasmResponse);
      case 'dataContract':
        return this.convertDataContractResponse(wasmResponse);
      case 'stateTransition':
        return this.convertStateTransitionResponse(wasmResponse);
      default:
        return wasmResponse;
    }
  }

  private convertIdentityResponse(wasmIdentity: any): any {
    // Convert wasm-sdk identity to js-dash-sdk Identity instance
    if (!wasmIdentity) return null;
    
    // If the platform has DPP available, use it to create proper Identity instance
    if (this.platform && this.platform.dpp) {
      try {
        // Convert from wasm-sdk format to js-dash-sdk Identity
        const identityData = {
          protocolVersion: wasmIdentity.protocolVersion || 1,
          id: wasmIdentity.id,
          publicKeys: wasmIdentity.publicKeys || [],
          balance: wasmIdentity.balance || 0,
          revision: wasmIdentity.revision || 0,
        };
        
        return this.platform.dpp.identity.create(
          identityData.id,
          identityData.publicKeys,
          identityData.balance,
          identityData.revision
        );
      } catch (e) {
        // Fallback to raw object if DPP creation fails
        return wasmIdentity;
      }
    }
    
    return wasmIdentity;
  }

  private convertDocumentResponse(wasmDocument: any): any {
    // Convert wasm-sdk document to js-dash-sdk Document instance
    if (!wasmDocument) return null;
    
    // If the platform has DPP available, use it to create proper Document instance
    if (this.platform && this.platform.dpp) {
      try {
        // For now, return the raw document as the platform will handle conversion
        // when it has the data contract context
        return wasmDocument;
      } catch (e) {
        // Fallback to raw object
        return wasmDocument;
      }
    }
    
    return wasmDocument;
  }

  private convertDataContractResponse(wasmContract: any): any {
    // Convert wasm-sdk data contract to js-dash-sdk DataContract instance
    if (!wasmContract) return null;
    
    // If the platform has DPP available, use it to create proper DataContract instance
    if (this.platform && this.platform.dpp) {
      try {
        // Create data contract from the wasm response
        const contractData = {
          protocolVersion: wasmContract.protocolVersion || 1,
          $id: wasmContract.id,
          $schema: wasmContract.schema || 'https://schema.dash.org/dpp-0-4-0/meta/data-contract',
          version: wasmContract.version || 1,
          ownerId: wasmContract.ownerId,
          documents: wasmContract.documents || wasmContract.documentSchemas || {},
        };
        
        return this.platform.dpp.dataContract.createFromObject(contractData);
      } catch (e) {
        // Fallback to raw object if DPP creation fails
        return wasmContract;
      }
    }
    
    return wasmContract;
  }

  private convertStateTransitionResponse(wasmResponse: any): any {
    // Convert wasm-sdk state transition response
    return {
      success: wasmResponse.success,
      data: wasmResponse.data,
      error: wasmResponse.error,
    };
  }

  /**
   * Get cached data if available and not expired
   */
  private getCached(key: string): any | null {
    const cached = this.cache.get(key);
    if (cached && Date.now() - cached.timestamp < this.cacheTTL) {
      return cached.data;
    }
    // Remove expired entry
    if (cached) {
      this.cache.delete(key);
    }
    return null;
  }

  /**
   * Set cache data
   */
  private setCache(key: string, data: any): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });
  }

  /**
   * Clear all cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Set cache TTL
   */
  setCacheTTL(ttl: number): void {
    this.cacheTTL = ttl;
  }

  /**
   * Cached query wrapper
   */
  async cachedQuery(key: string, queryFn: () => Promise<any>): Promise<any> {
    // Check cache first
    const cached = this.getCached(key);
    if (cached !== null) {
      return cached;
    }

    // Execute query and cache result
    const result = await queryFn();
    if (result !== null && result !== undefined) {
      this.setCache(key, result);
    }

    return result;
  }

  /**
   * Clean up resources
   */
  async dispose(): Promise<void> {
    this.sdkInstance = undefined;
    this.wasmSdk = undefined;
    this.initialized = false;
    this.cache.clear();
  }
}