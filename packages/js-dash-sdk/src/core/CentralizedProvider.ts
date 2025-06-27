import { AbstractContextProvider } from './ContextProvider';

interface CentralizedProviderOptions {
  url: string;
  apiKey?: string;
  cacheDuration?: number;
}

interface PlatformStatus {
  blockHeight: number;
  blockTime: number;
  coreChainLockedHeight: number;
  version: string;
  timePerBlock: number;
  epoch: number;
}

export class CentralizedProvider extends AbstractContextProvider {
  private url: string;
  private apiKey?: string;
  private headers: Record<string, string>;

  constructor(options: CentralizedProviderOptions) {
    super();
    this.url = options.url.replace(/\/$/, ''); // Remove trailing slash
    this.apiKey = options.apiKey;
    this.cacheDuration = options.cacheDuration || 5000;
    
    this.headers = {
      'Content-Type': 'application/json',
    };
    
    if (this.apiKey) {
      this.headers['Authorization'] = `Bearer ${this.apiKey}`;
    }
  }

  private async fetch<T>(endpoint: string, params?: any): Promise<T> {
    const url = new URL(`${this.url}${endpoint}`);
    
    if (params) {
      Object.keys(params).forEach(key => 
        url.searchParams.append(key, params[key])
      );
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: this.headers,
    });

    if (!response.ok) {
      throw new Error(`Context provider request failed: ${response.status} ${response.statusText}`);
    }

    return response.json();
  }

  async getLatestPlatformBlockHeight(): Promise<number> {
    const cached = this.getCached<number>('blockHeight');
    if (cached !== null) return cached;

    const status = await this.fetch<PlatformStatus>('/status');
    this.setCache('blockHeight', status.blockHeight);
    this.setCache('blockTime', status.blockTime);
    this.setCache('coreChainLockedHeight', status.coreChainLockedHeight);
    this.setCache('version', status.version);
    this.setCache('timePerBlock', status.timePerBlock);
    
    return status.blockHeight;
  }

  async getLatestPlatformBlockTime(): Promise<number> {
    const cached = this.getCached<number>('blockTime');
    if (cached !== null) return cached;

    const status = await this.fetch<PlatformStatus>('/status');
    this.setCache('blockTime', status.blockTime);
    return status.blockTime;
  }

  async getLatestPlatformCoreChainLockedHeight(): Promise<number> {
    const cached = this.getCached<number>('coreChainLockedHeight');
    if (cached !== null) return cached;

    const status = await this.fetch<PlatformStatus>('/status');
    this.setCache('coreChainLockedHeight', status.coreChainLockedHeight);
    return status.coreChainLockedHeight;
  }

  async getLatestPlatformVersion(): Promise<string> {
    const cached = this.getCached<string>('version');
    if (cached !== null) return cached;

    const status = await this.fetch<PlatformStatus>('/status');
    this.setCache('version', status.version);
    return status.version;
  }

  async getProposerBlockCount(proposerProTxHash: string): Promise<number | null> {
    const cacheKey = `proposerBlockCount:${proposerProTxHash}`;
    const cached = this.getCached<number>(cacheKey);
    if (cached !== null) return cached;

    try {
      const result = await this.fetch<{ count: number }>('/proposer/block-count', {
        proposerProTxHash
      });
      this.setCache(cacheKey, result.count);
      return result.count;
    } catch {
      return null;
    }
  }

  async getTimePerBlockMillis(): Promise<number> {
    const cached = this.getCached<number>('timePerBlock');
    if (cached !== null) return cached;

    const status = await this.fetch<PlatformStatus>('/status');
    this.setCache('timePerBlock', status.timePerBlock);
    return status.timePerBlock;
  }

  async getBlockProposer(blockHeight: number): Promise<string | null> {
    const cacheKey = `blockProposer:${blockHeight}`;
    const cached = this.getCached<string>(cacheKey);
    if (cached !== null) return cached;

    try {
      const result = await this.fetch<{ proposer: string }>('/block/proposer', {
        height: blockHeight
      });
      this.setCache(cacheKey, result.proposer);
      return result.proposer;
    } catch {
      return null;
    }
  }
}