/**
 * Factory for creating context providers with common configurations
 */

import { ContextProvider } from '../core/types';
import { BluetoothProvider } from '../bluetooth/BluetoothProvider';
import { BluetoothConnection } from '../bluetooth/BluetoothConnection';
import { CentralizedProvider } from '../core/CentralizedProvider';
import { WebServiceProvider } from './WebServiceProvider';
import { PriorityContextProvider } from './PriorityContextProvider';
import { ProviderCapability } from './types';

export interface ProviderFactoryOptions {
  network?: 'mainnet' | 'testnet' | 'devnet';
  
  // Provider selection
  providers?: Array<'bluetooth' | 'webservice' | 'centralized'>;
  customProviders?: Array<{
    provider: ContextProvider;
    priority: number;
    name: string;
  }>;
  
  // Priority configuration
  usePriority?: boolean;
  priorities?: {
    bluetooth?: number;
    webservice?: number;
    centralized?: number;
  };
  
  // Provider-specific options
  bluetooth?: {
    requireAuthentication?: boolean;
    autoReconnect?: boolean;
  };
  webservice?: {
    url?: string;
    apiKey?: string;
    cacheDuration?: number;
  };
  centralized?: {
    url?: string;
    apiKey?: string;
  };
  
  // Priority provider options
  fallbackEnabled?: boolean;
  cacheResults?: boolean;
  logErrors?: boolean;
}

export class ProviderFactory {
  /**
   * Create a context provider based on options
   */
  static async create(options: ProviderFactoryOptions = {}): Promise<ContextProvider> {
    const network = options.network || 'testnet';
    
    // Default provider selection
    const providers = options.providers || ['webservice', 'centralized'];
    
    // Default priorities (higher number = higher priority)
    const defaultPriorities = {
      bluetooth: 100,    // Highest - most secure, user's device
      webservice: 80,    // High - dedicated service
      centralized: 60,   // Medium - fallback option
    };
    
    const priorities = { ...defaultPriorities, ...options.priorities };
    
    // Single provider mode
    if (!options.usePriority && providers.length === 1) {
      return this.createSingleProvider(providers[0], network, options);
    }
    
    // Priority provider mode
    const providerEntries = [];
    
    // Add requested providers
    for (const providerType of providers) {
      try {
        const provider = await this.createSingleProvider(providerType, network, options);
        providerEntries.push({
          provider,
          priority: priorities[providerType] || 50,
          name: this.getProviderName(providerType),
          capabilities: this.getProviderCapabilities(providerType),
        });
      } catch (error) {
        console.warn(`Failed to create ${providerType} provider:`, error);
      }
    }
    
    // Add custom providers
    if (options.customProviders) {
      providerEntries.push(...options.customProviders);
    }
    
    // If only one provider was successfully created, return it directly
    if (providerEntries.length === 1 && !options.usePriority) {
      return providerEntries[0].provider;
    }
    
    // Create priority provider
    return new PriorityContextProvider({
      providers: providerEntries,
      fallbackEnabled: options.fallbackEnabled ?? true,
      cacheResults: options.cacheResults ?? true,
      logErrors: options.logErrors ?? false,
    });
  }
  
  /**
   * Create a provider with Bluetooth as primary and web service as fallback
   */
  static async createWithBluetooth(options: ProviderFactoryOptions = {}): Promise<ContextProvider> {
    return this.create({
      ...options,
      providers: ['bluetooth', 'webservice', 'centralized'],
      usePriority: true,
      priorities: {
        bluetooth: 100,
        webservice: 80,
        centralized: 60,
        ...options.priorities,
      },
    });
  }
  
  /**
   * Create a provider with web service as primary
   */
  static createWithWebService(options: ProviderFactoryOptions = {}): Promise<ContextProvider> {
    return this.create({
      ...options,
      providers: ['webservice', 'centralized'],
      usePriority: true,
      priorities: {
        webservice: 100,
        centralized: 80,
        ...options.priorities,
      },
    });
  }
  
  /**
   * Create a single provider instance
   */
  private static async createSingleProvider(
    type: 'bluetooth' | 'webservice' | 'centralized',
    network: string,
    options: ProviderFactoryOptions
  ): Promise<ContextProvider> {
    switch (type) {
      case 'bluetooth': {
        if (!BluetoothConnection.isAvailable()) {
          throw new Error('Bluetooth is not available in this environment');
        }
        
        const bluetoothProvider = new BluetoothProvider({
          requireAuthentication: options.bluetooth?.requireAuthentication ?? true,
          autoReconnect: options.bluetooth?.autoReconnect ?? true,
        });
        
        // Attempt to connect
        await bluetoothProvider.connect();
        
        return bluetoothProvider;
      }
      
      case 'webservice': {
        return new WebServiceProvider({
          network: network as 'mainnet' | 'testnet',
          url: options.webservice?.url,
          apiKey: options.webservice?.apiKey,
          cacheDuration: options.webservice?.cacheDuration,
        });
      }
      
      case 'centralized': {
        const urls: Record<string, string> = {
          mainnet: 'https://platform.dash.org/api',
          testnet: 'https://platform-testnet.dash.org/api',
          devnet: 'https://platform-devnet.dash.org/api',
        };
        
        return new CentralizedProvider({
          url: options.centralized?.url || urls[network] || urls.testnet,
          apiKey: options.centralized?.apiKey,
        });
      }
      
      default:
        throw new Error(`Unknown provider type: ${type}`);
    }
  }
  
  private static getProviderName(type: string): string {
    const names: Record<string, string> = {
      bluetooth: 'BluetoothProvider',
      webservice: 'WebServiceProvider',
      centralized: 'CentralizedProvider',
    };
    return names[type] || type;
  }
  
  private static getProviderCapabilities(type: string): ProviderCapability[] {
    const capabilities: Record<string, ProviderCapability[]> = {
      bluetooth: [
        ProviderCapability.PLATFORM_STATE,
        ProviderCapability.QUORUM_KEYS,
        ProviderCapability.BLOCK_PROPOSER,
        ProviderCapability.SIGNING, // Bluetooth can also sign
      ],
      webservice: [
        ProviderCapability.PLATFORM_STATE,
        ProviderCapability.QUORUM_KEYS,
        ProviderCapability.BLOCK_PROPOSER,
      ],
      centralized: [
        ProviderCapability.PLATFORM_STATE,
        ProviderCapability.BLOCK_PROPOSER,
      ],
    };
    return capabilities[type] || [ProviderCapability.PLATFORM_STATE];
  }
}