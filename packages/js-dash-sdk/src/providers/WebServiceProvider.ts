/**
 * Web service context provider that fetches platform state and quorum keys
 */

import { AbstractContextProvider } from '../core/ContextProvider';
import { 
  WebServiceProviderOptions,
  QuorumInfo,
  QuorumServiceResponse,
  ProviderCapability,
  ProviderWithCapabilities
} from './types';

export class WebServiceProvider extends AbstractContextProvider implements ProviderWithCapabilities {
  private url: string;
  private apiKey?: string;
  private headers: Record<string, string>;
  private timeout: number;
  private retryAttempts: number;
  private retryDelay: number;
  private quorumCache = new Map<string, QuorumInfo>();
  
  constructor(options: WebServiceProviderOptions = {}) {
    super();
    
    // Set URL based on network
    if (options.url) {
      this.url = options.url.replace(/\/$/, '');
    } else {
      const network = options.network || 'testnet';
      this.url = network === 'mainnet' 
        ? 'https://quorum.networks.dash.org'
        : 'https://quorum.testnet.networks.dash.org';
    }
    
    this.apiKey = options.apiKey;
    this.timeout = options.timeout || 30000;
    this.cacheDuration = options.cacheDuration || 60000; // 1 minute
    this.retryAttempts = options.retryAttempts || 3;
    this.retryDelay = options.retryDelay || 1000;
    
    this.headers = {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    };
    
    if (this.apiKey) {
      this.headers['Authorization'] = `Bearer ${this.apiKey}`;
    }
  }
  
  getName(): string {
    return 'WebServiceProvider';
  }
  
  getCapabilities(): ProviderCapability[] {
    return [
      ProviderCapability.PLATFORM_STATE,
      ProviderCapability.QUORUM_KEYS,
      ProviderCapability.BLOCK_PROPOSER,
    ];
  }
  
  async isAvailable(): Promise<boolean> {
    try {
      // Try to fetch platform status
      await this.fetchWithRetry('/status');
      return true;
    } catch {
      return false;
    }
  }
  
  async getLatestPlatformBlockHeight(): Promise<number> {
    const cached = this.getCached<number>('blockHeight');
    if (cached !== null) return cached;
    
    const response = await this.fetchWithRetry('/status');
    const data = await response.json();
    
    const height = data.platform?.blockHeight || data.blockHeight;
    if (typeof height !== 'number') {
      throw new Error('Invalid block height response');
    }
    
    this.setCache('blockHeight', height);
    return height;
  }
  
  async getLatestPlatformBlockTime(): Promise<number> {
    const cached = this.getCached<number>('blockTime');
    if (cached !== null) return cached;
    
    const response = await this.fetchWithRetry('/status');
    const data = await response.json();
    
    const time = data.platform?.blockTime || data.blockTime;
    if (typeof time !== 'number') {
      throw new Error('Invalid block time response');
    }
    
    this.setCache('blockTime', time);
    return time;
  }
  
  async getLatestPlatformCoreChainLockedHeight(): Promise<number> {
    const cached = this.getCached<number>('coreChainLockedHeight');
    if (cached !== null) return cached;
    
    const response = await this.fetchWithRetry('/status');
    const data = await response.json();
    
    const height = data.platform?.coreChainLockedHeight || data.coreChainLockedHeight;
    if (typeof height !== 'number') {
      throw new Error('Invalid core chain locked height response');
    }
    
    this.setCache('coreChainLockedHeight', height);
    return height;
  }
  
  async getLatestPlatformVersion(): Promise<string> {
    const cached = this.getCached<string>('version');
    if (cached !== null) return cached;
    
    const response = await this.fetchWithRetry('/status');
    const data = await response.json();
    
    const version = data.platform?.version || data.version || '1.0.0';
    this.setCache('version', version);
    return version;
  }
  
  async getProposerBlockCount(proposerProTxHash: string): Promise<number | null> {
    const cacheKey = `proposerBlockCount:${proposerProTxHash}`;
    const cached = this.getCached<number>(cacheKey);
    if (cached !== null) return cached;
    
    try {
      const response = await this.fetchWithRetry(`/proposers/${proposerProTxHash}/blocks/count`);
      const data = await response.json();
      
      const count = data.count;
      if (typeof count === 'number') {
        this.setCache(cacheKey, count);
        return count;
      }
    } catch {
      // Not all providers support this endpoint
    }
    
    return null;
  }
  
  async getTimePerBlockMillis(): Promise<number> {
    const cached = this.getCached<number>('timePerBlock');
    if (cached !== null) return cached;
    
    // Default to 2.5 seconds if not provided by service
    const defaultTime = 2500;
    
    try {
      const response = await this.fetchWithRetry('/status');
      const data = await response.json();
      
      const time = data.platform?.timePerBlock || data.timePerBlock || defaultTime;
      this.setCache('timePerBlock', time);
      return time;
    } catch {
      return defaultTime;
    }
  }
  
  async getBlockProposer(blockHeight: number): Promise<string | null> {
    const cacheKey = `blockProposer:${blockHeight}`;
    const cached = this.getCached<string>(cacheKey);
    if (cached !== null) return cached;
    
    try {
      const response = await this.fetchWithRetry(`/blocks/${blockHeight}/proposer`);
      const data = await response.json();
      
      const proposer = data.proposer || data.proposerProTxHash;
      if (typeof proposer === 'string') {
        this.setCache(cacheKey, proposer);
        return proposer;
      }
    } catch {
      // Not all providers support this endpoint
    }
    
    return null;
  }
  
  /**
   * Get quorum public keys
   */
  async getQuorumKeys(): Promise<Map<string, QuorumInfo>> {
    const cacheKey = 'quorumKeys';
    const cached = this.getCached<Map<string, QuorumInfo>>(cacheKey);
    if (cached !== null) return cached;
    
    try {
      const response = await this.fetchWithRetry('/quorums');
      const data: QuorumServiceResponse = await response.json();
      
      const quorumMap = new Map<string, QuorumInfo>();
      
      for (const [quorumHash, quorumData] of Object.entries(data)) {
        quorumMap.set(quorumHash, {
          quorumHash,
          quorumPublicKey: {
            version: quorumData.version || 1,
            publicKey: quorumData.publicKey,
            type: (quorumData.type as 'ECDSA' | 'BLS') || 'BLS',
          },
          isActive: true,
        });
      }
      
      this.setCache(cacheKey, quorumMap);
      this.quorumCache = quorumMap;
      return quorumMap;
    } catch (error) {
      // Return cached data if available
      if (this.quorumCache.size > 0) {
        return this.quorumCache;
      }
      throw error;
    }
  }
  
  /**
   * Get a specific quorum by hash
   */
  async getQuorum(quorumHash: string): Promise<QuorumInfo | null> {
    const quorums = await this.getQuorumKeys();
    return quorums.get(quorumHash) || null;
  }
  
  /**
   * Get active quorums
   */
  async getActiveQuorums(): Promise<QuorumInfo[]> {
    const quorums = await this.getQuorumKeys();
    return Array.from(quorums.values()).filter(q => q.isActive);
  }
  
  /**
   * Fetch with retry logic
   */
  private async fetchWithRetry(endpoint: string, options?: RequestInit): Promise<Response> {
    let lastError: Error | null = null;
    
    for (let attempt = 0; attempt <= this.retryAttempts; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);
        
        const response = await fetch(`${this.url}${endpoint}`, {
          ...options,
          headers: { ...this.headers, ...options?.headers },
          signal: controller.signal,
        });
        
        clearTimeout(timeoutId);
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}: ${response.statusText}`);
        }
        
        return response;
      } catch (error: any) {
        lastError = error;
        
        // Don't retry on client errors
        if (error.message?.includes('HTTP 4')) {
          throw error;
        }
        
        // Wait before retry with exponential backoff
        if (attempt < this.retryAttempts) {
          await new Promise(resolve => 
            setTimeout(resolve, this.retryDelay * Math.pow(2, attempt))
          );
        }
      }
    }
    
    throw new Error(`Failed after ${this.retryAttempts + 1} attempts: ${lastError?.message}`);
  }
}