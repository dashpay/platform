import { EvonodesProvider } from './EvonodesProvider';
import { loadWasmSdk } from './WasmLoader';

export interface WasmContextProviderOptions {
  network: 'mainnet' | 'testnet' | 'devnet';
  evonodesProvider?: EvonodesProvider;
  addressList?: string[];
  timeout?: number;
}

/**
 * Creates a WASM context provider configured with dynamic evonodes
 * This provider bridges the EvonodesProvider with the WASM SDK's context requirements
 */
export class WasmContextProvider {
  private network: 'mainnet' | 'testnet' | 'devnet';
  private evonodesProvider: EvonodesProvider;
  private addressList?: string[];
  private timeout: number;

  constructor(options: WasmContextProviderOptions) {
    this.network = options.network;
    this.evonodesProvider = options.evonodesProvider || new EvonodesProvider();
    this.addressList = options.addressList;
    this.timeout = options.timeout || 30000; // 30 seconds default
  }

  /**
   * Creates a WASM SDK builder configured with dynamic evonodes
   * @returns The configured WasmSdkBuilder instance
   */
  async createWasmSdkBuilder(): Promise<any> {
    // Load WASM module
    const wasm = await loadWasmSdk();
    
    if (!wasm.WasmSdkBuilder) {
      throw new Error('WasmSdkBuilder not found in WASM module');
    }

    // Create builder for the specified network
    let builder;
    if (this.network === 'mainnet') {
      builder = wasm.WasmSdkBuilder.new_mainnet();
    } else {
      // Use testnet builder for both testnet and devnet
      builder = wasm.WasmSdkBuilder.new_testnet();
    }

    // Set timeout if method is available
    if (typeof builder.withTimeout === 'function') {
      builder.withTimeout(this.timeout);
    } else if (typeof builder.with_timeout === 'function') {
      builder.with_timeout(this.timeout);
    }

    // Configure evonodes
    await this.configureEvonodes(builder);

    return builder;
  }

  /**
   * Configures the WASM SDK builder with evonodes
   * @param builder The WasmSdkBuilder instance to configure
   */
  private async configureEvonodes(builder: any): Promise<void> {
    try {
      // Use provided address list or fetch from network
      let evonodes: string[];
      
      if (this.addressList && this.addressList.length > 0) {
        evonodes = this.addressList;
        console.log(`Using provided address list with ${evonodes.length} nodes`);
      } else {
        // Fetch current evonodes from the network
        const networkType = this.network === 'devnet' ? 'testnet' : this.network;
        evonodes = await this.evonodesProvider.getEvonodes(networkType as 'mainnet' | 'testnet');
        console.log(`Fetched ${evonodes.length} active evonodes from ${networkType} network`);
      }

      // Try different methods to configure endpoints
      const configured = await this.tryConfigureEndpoints(builder, evonodes);
      
      if (!configured) {
        console.warn('No method found to configure custom evonodes, using default configuration');
      }
    } catch (error) {
      console.error('Failed to configure evonodes:', error);
      console.warn('Using default evonode configuration');
    }
  }

  /**
   * Tries different methods to configure endpoints on the builder
   * @param builder The WasmSdkBuilder instance
   * @param evonodes List of evonode addresses
   * @returns True if configuration was successful
   */
  private async tryConfigureEndpoints(builder: any, evonodes: string[]): Promise<boolean> {
    // Method 1: Try with_address_list (expects full URLs)
    if (typeof builder.with_address_list === 'function') {
      console.log('Configuring with with_address_list method');
      const urls = evonodes.map(node => {
        // Ensure proper URL format
        if (!node.startsWith('http://') && !node.startsWith('https://')) {
          return `https://${node}`;
        }
        return node;
      });
      builder.with_address_list(urls);
      console.log(`Configured SDK with ${urls.length} evonodes`);
      return true;
    }

    // Method 2: Try add_dashmate_endpoint (individual endpoints)
    if (typeof builder.add_dashmate_endpoint === 'function') {
      console.log('Configuring with add_dashmate_endpoint method');
      // Limit to prevent overload
      const maxNodes = Math.min(evonodes.length, 10);
      for (let i = 0; i < maxNodes; i++) {
        const node = evonodes[i];
        const url = node.startsWith('http') ? node : `https://${node}`;
        builder.add_dashmate_endpoint(url, true); // true for SSL
        console.log(`Added evonode ${i + 1}/${maxNodes}: ${url}`);
      }
      return true;
    }

    // Method 3: Try add_evonode (raw addresses)
    if (typeof builder.add_evonode === 'function') {
      console.log('Configuring with add_evonode method');
      const maxNodes = Math.min(evonodes.length, 10);
      for (let i = 0; i < maxNodes; i++) {
        builder.add_evonode(evonodes[i]);
        console.log(`Added evonode ${i + 1}/${maxNodes}: ${evonodes[i]}`);
      }
      return true;
    }

    // Method 4: Try with_core_ip_list (if available)
    if (typeof builder.with_core_ip_list === 'function') {
      console.log('Configuring with with_core_ip_list method');
      builder.with_core_ip_list(evonodes);
      console.log(`Configured SDK with ${evonodes.length} evonodes`);
      return true;
    }

    return false;
  }

  /**
   * Creates a fully initialized WASM SDK instance
   * @returns The built WasmSdk instance
   */
  async createWasmSdk(): Promise<any> {
    const builder = await this.createWasmSdkBuilder();
    console.log('Building WASM SDK instance...');
    const sdk = builder.build();
    console.log('WASM SDK instance created successfully');
    return sdk;
  }

  /**
   * Gets the current network
   */
  getNetwork(): string {
    return this.network;
  }

  /**
   * Updates the address list
   * @param addresses New list of evonode addresses
   */
  setAddressList(addresses: string[]): void {
    this.addressList = addresses;
  }

  /**
   * Clears the evonode cache to force a refresh
   */
  clearCache(): void {
    this.evonodesProvider.clearCache(this.network as 'mainnet' | 'testnet');
  }
}

/**
 * Factory function to create a WasmContextProvider
 * @param network The network to connect to
 * @param options Optional configuration
 * @returns A configured WasmContextProvider instance
 */
export function createWasmContextProvider(
  network: 'mainnet' | 'testnet' | 'devnet',
  options?: Partial<WasmContextProviderOptions>
): WasmContextProvider {
  return new WasmContextProvider({
    network,
    ...options
  });
}

/**
 * Creates a WASM SDK instance with dynamic evonode configuration
 * This is a convenience function that combines context provider creation and SDK building
 * @param network The network to connect to
 * @param options Optional configuration
 * @returns A configured WasmSdk instance
 */
export async function createWasmSdkWithDynamicEvonodes(
  network: 'mainnet' | 'testnet' | 'devnet',
  options?: Partial<WasmContextProviderOptions>
): Promise<any> {
  const provider = createWasmContextProvider(network, options);
  return provider.createWasmSdk();
}