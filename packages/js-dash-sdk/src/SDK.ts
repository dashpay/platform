import { EventEmitter } from 'eventemitter3';
import { 
  SDKOptions, 
  Network, 
  ContextProvider,
  AppDefinition 
} from './core/types';
import { CentralizedProvider } from './core/CentralizedProvider';
import { loadWasmSdk, getWasmSdk } from './core/WasmLoader';

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
    // Use WebServiceProvider as default with CentralizedProvider as fallback
    const { PriorityContextProvider } = require('./providers/PriorityContextProvider');
    const { WebServiceProvider } = require('./providers/WebServiceProvider');
    
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
    
    // Create priority provider with web service as primary
    return new PriorityContextProvider({
      providers: [
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
      return;
    }

    // Load WASM SDK
    const wasm = await loadWasmSdk();
    
    // Initialize WASM SDK instance
    this.wasmSdk = new wasm.WasmSdk(this.network.type);
    
    // Verify context provider is working
    const isValid = await this.contextProvider.isValid();
    if (!isValid) {
      throw new Error('Context provider is not valid or cannot connect to the network');
    }
    
    this.initialized = true;
    this.emit('initialized');
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