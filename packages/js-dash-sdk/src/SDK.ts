import { EventEmitter } from 'eventemitter3';
import { 
  SDKOptions, 
  Network, 
  ContextProvider,
  AppDefinition 
} from './core/types';
import { CentralizedProvider } from './core/CentralizedProvider';
import { loadWasmSdk, getWasmSdk } from './core/WasmLoader';
import { createWasmSdkWithDynamicEvonodes } from './core/WasmContextProvider';

// Re-export types for external use
export type { SDKOptions, Network, ContextProvider, AppDefinition };

export class SDK extends EventEmitter {
  private options: SDKOptions;
  private contextProvider: ContextProvider;
  private wasmSdk: any;
  private network: Network;
  private apps: Record<string, AppDefinition> = {};
  private initialized = false;

  constructor(options: SDKOptions = {}) {
    super();
    this.options = options;
    
    // Set network
    this.network = this.parseNetwork(options.network);
    
    // Set context provider
    this.contextProvider = options.contextProvider || this.createDefaultProvider();
    
    // Set apps
    if (options.apps) {
      this.apps = options.apps;
    }
  }

  private parseNetwork(network?: Network | string): Network {
    if (!network) {
      return { name: 'testnet', type: 'testnet' };
    }
    
    if (typeof network === 'string') {
      const type = network as 'mainnet' | 'testnet' | 'devnet';
      return { name: network, type };
    }
    
    return network;
  }

  private createDefaultProvider(): ContextProvider {
    // Use custom masternode provider as primary
    const { PriorityContextProvider } = require('./providers/PriorityContextProvider');
    const { CustomMasternodeProvider } = require('./providers/CustomMasternodeProvider');
    const { WebServiceProvider } = require('./providers/WebServiceProvider');
    
    const customMasternodeProvider = new CustomMasternodeProvider();
    
    const webServiceProvider = new WebServiceProvider({
      network: this.network.type as 'mainnet' | 'testnet'
    });
    
    const urls: Record<string, string> = {
      mainnet: 'https://platform.dash.org/api',
      testnet: 'https://platform-testnet.dash.org/api',
      devnet: 'https://platform-devnet.dash.org/api',
    };
    
    const url = urls[this.network.type] || urls.testnet;
    const centralizedProvider = new CentralizedProvider({ url });
    
    // Create priority provider with custom masternodes as primary
    return new PriorityContextProvider({
      providers: [
        {
          provider: customMasternodeProvider,
          priority: 150,
          name: 'CustomMasternodeProvider'
        },
        {
          provider: webServiceProvider,
          priority: 100,
          name: 'WebServiceProvider'
        },
        {
          provider: centralizedProvider,
          priority: 80,
          name: 'CentralizedProvider'
        }
      ],
      fallbackEnabled: true,
      cacheResults: true
    });
  }

  async initialize(): Promise<void> {
    if (this.initialized) {
      console.log('SDK already initialized, skipping...');
      return;
    }

    console.log('=== SDK Initialization Started ===');
    console.log('Network:', this.network);
    
    try {
      // Load WASM SDK module first
      console.log('Loading WASM SDK module...');
      await loadWasmSdk();
      
      // Create WASM SDK with dynamic evonodes
      console.log(`Creating WASM SDK for network: ${this.network.type}`);
      this.wasmSdk = await createWasmSdkWithDynamicEvonodes(
        this.network.type as 'mainnet' | 'testnet', 
        {
          timeout: 30000 // 30 second timeout
        }
      );
      
      console.log('WASM SDK instance built successfully');
      console.log('WASM SDK type:', this.wasmSdk?.constructor?.name);
      console.log('WASM SDK methods:', Object.getOwnPropertyNames(Object.getPrototypeOf(this.wasmSdk || {})).slice(0, 10));
      
      // Skip context provider validation since WASM SDK handles its own connections
      console.log('WASM SDK initialized with Dash testnet evonodes');
      
      this.initialized = true;
      this.emit('initialized');
      
      console.log('=== SDK Initialization Complete ===');
    } catch (error) {
      console.error('=== SDK Initialization Failed ===');
      console.error('Error:', error);
      throw error;
    }
  }

  isInitialized(): boolean {
    return this.initialized;
  }

  getNetwork(): Network {
    return this.network;
  }

  getContextProvider(): ContextProvider {
    return this.contextProvider;
  }

  getWasmSdk(): any {
    if (!this.initialized) {
      throw new Error('SDK not initialized. Call initialize() first.');
    }
    return this.wasmSdk;
  }

  getWasmModule(): any {
    if (!this.initialized) {
      throw new Error('SDK not initialized. Call initialize() first.');
    }
    return getWasmSdk();
  }

  registerApp(name: string, definition: AppDefinition): void {
    this.apps[name] = definition;
    this.emit('app:registered', { name, definition });
  }

  getApp(name: string): AppDefinition | undefined {
    return this.apps[name];
  }

  getApps(): Record<string, AppDefinition> {
    return { ...this.apps };
  }

  hasApp(name: string): boolean {
    return name in this.apps;
  }

  getOptions(): SDKOptions {
    return this.options;
  }

  // Utility method to create context for WASM calls
  async createContext() {
    const [
      blockHeight,
      blockTime,
      coreChainLockedHeight,
      version
    ] = await Promise.all([
      this.contextProvider.getLatestPlatformBlockHeight(),
      this.contextProvider.getLatestPlatformBlockTime(),
      this.contextProvider.getLatestPlatformCoreChainLockedHeight(),
      this.contextProvider.getLatestPlatformVersion()
    ]);

    return {
      blockHeight,
      blockTime,
      coreChainLockedHeight,
      version
    };
  }

  destroy(): void {
    this.removeAllListeners();
    this.initialized = false;
  }
}