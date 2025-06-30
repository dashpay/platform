interface MasternodeInfo {
  proTxHash: string;
  address: string;
  payee: string;
  status: string;
  type: string;
  posePenalty: number;
  registeredHeight: number;
  lastPaidHeight: number;
  nextPaymentHeight: number;
  ownerAddress: string;
  votingAddress: string;
  isValid: boolean;
  extraPayload: {
    version: number;
    service: string;
    operatorReward: number;
    platformNodeID?: string;
    platformP2PPort?: number;
    platformHTTPPort?: number;
  };
}

interface EvonodesCache {
  nodes: string[];
  timestamp: number;
}

export class EvonodesProvider {
  private static readonly CACHE_DURATION = 5 * 60 * 1000; // 5 minutes in milliseconds
  private static readonly GRPC_WEB_PORT = 1443;
  private static readonly DASH_PORT = 19999;
  
  private static readonly ENDPOINTS = {
    testnet: 'https://quorums.testnet.networks.dash.org/masternodes',
    mainnet: 'https://quorums.mainnet.networks.dash.org/masternodes'
  };
  
  private static readonly FALLBACK_NODES = {
    testnet: [
      // All ENABLED nodes from the list, port 19999 -> 1443
      '52.13.132.146:1443',    // Valid SSL cert
      '52.89.154.48:1443',
      '44.227.137.77:1443',
      '52.40.219.41:1443',
      '54.149.33.167:1443',
      '54.187.14.232:1443',
      '52.12.176.90:1443',
      '52.34.144.50:1443',
      '44.239.39.153:1443',
      '34.214.48.68:1443',
      '35.82.197.197:1443',
      '35.167.145.149:1443',
      '52.42.202.128:1443',
      '35.163.144.230:1443',
      '44.228.242.181:1443',
      '54.201.32.131:1443',
      '35.164.23.245:1443',
      '52.43.13.92:1443',
      '52.24.124.162:1443',
      '54.68.235.201:1443',
      '52.10.229.11:1443',
      '44.240.98.102:1443',
      '52.33.28.47:1443',
      '35.85.21.179:1443',
      '52.43.86.231:1443'
    ],
    mainnet: [] // Add mainnet fallback nodes when available
  };
  
  private cache: Map<string, EvonodesCache> = new Map();
  
  /**
   * Fetches the list of evonodes for the specified network
   * @param network - The network to fetch nodes for ('testnet' or 'mainnet')
   * @returns Array of evonode addresses in format "ip:port"
   */
  async getEvonodes(network: 'testnet' | 'mainnet'): Promise<string[]> {
    // Check cache first
    const cached = this.cache.get(network);
    if (cached && Date.now() - cached.timestamp < EvonodesProvider.CACHE_DURATION) {
      return cached.nodes;
    }
    
    try {
      // Fetch from network endpoint
      const nodes = await this.fetchEvonodesFromNetwork(network);
      
      // Cache the results
      this.cache.set(network, {
        nodes,
        timestamp: Date.now()
      });
      
      return nodes;
    } catch (error) {
      console.error(`Failed to fetch evonodes for ${network}:`, error);
      
      // Return fallback nodes
      return EvonodesProvider.FALLBACK_NODES[network];
    }
  }
  
  /**
   * Fetches evonodes from the network endpoint
   * @param network - The network to fetch nodes for
   * @returns Array of evonode addresses
   */
  private async fetchEvonodesFromNetwork(network: 'testnet' | 'mainnet'): Promise<string[]> {
    const endpoint = EvonodesProvider.ENDPOINTS[network];
    
    const response = await fetch(endpoint, {
      method: 'GET',
      headers: {
        'Accept': 'application/json'
      },
      signal: AbortSignal.timeout(10000) // 10 second timeout
    });
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    
    const result = await response.json();
    
    // The response format is { success: boolean, data: array, message: string }
    if (!result.success || !Array.isArray(result.data)) {
      throw new Error(`Invalid response format: ${result.message || 'expected success with data array'}`);
    }
    
    // Filter and transform the nodes
    const evonodes = result.data
      .filter((node: MasternodeInfo) => this.isValidEvonode(node))
      .map((node: MasternodeInfo) => this.extractNodeAddress(node))
      .filter((address): address is string => address !== null);
    
    if (evonodes.length === 0) {
      throw new Error('No valid evonodes found');
    }
    
    return evonodes;
  }
  
  /**
   * Checks if a masternode is a valid evonode
   * @param node - The masternode info to check
   * @returns True if the node is a valid evonode
   */
  private isValidEvonode(node: MasternodeInfo): boolean {
    // Check if node is enabled
    if (node.status !== 'ENABLED') {
      return false;
    }
    
    // Check if node is valid
    if (!node.isValid) {
      return false;
    }
    
    // Check if it's an evonode (has platform fields)
    if (!node.extraPayload?.platformNodeID) {
      return false;
    }
    
    // Check if it has the required platform ports
    if (!node.extraPayload.platformP2PPort || !node.extraPayload.platformHTTPPort) {
      return false;
    }
    
    return true;
  }
  
  /**
   * Extracts the node address from masternode info
   * @param node - The masternode info
   * @returns The node address in format "ip:port" or null if invalid
   */
  private extractNodeAddress(node: MasternodeInfo): string | null {
    try {
      // Extract service address (format: "ip:port")
      const service = node.extraPayload?.service;
      if (!service) {
        return null;
      }
      
      // Parse IP and port
      const [ip, portStr] = service.split(':');
      const port = parseInt(portStr, 10);
      
      if (!ip || isNaN(port)) {
        return null;
      }
      
      // Convert port 19999 to gRPC-Web port 1443
      if (port === EvonodesProvider.DASH_PORT) {
        return `${ip}:${EvonodesProvider.GRPC_WEB_PORT}`;
      }
      
      // If it's already using a different port, keep it
      return service;
    } catch (error) {
      console.error('Failed to extract node address:', error);
      return null;
    }
  }
  
  /**
   * Clears the cache for a specific network or all networks
   * @param network - The network to clear cache for, or undefined to clear all
   */
  clearCache(network?: 'testnet' | 'mainnet'): void {
    if (network) {
      this.cache.delete(network);
    } else {
      this.cache.clear();
    }
  }
  
  /**
   * Gets the current cache status
   * @returns Object with cache information for each network
   */
  getCacheStatus(): Record<string, { isCached: boolean; age?: number }> {
    const status: Record<string, { isCached: boolean; age?: number }> = {};
    
    for (const network of ['testnet', 'mainnet'] as const) {
      const cached = this.cache.get(network);
      if (cached) {
        status[network] = {
          isCached: true,
          age: Date.now() - cached.timestamp
        };
      } else {
        status[network] = {
          isCached: false
        };
      }
    }
    
    return status;
  }
}

// Export a singleton instance for convenience
export const evonodesProvider = new EvonodesProvider();