import { ContextProvider } from './types';

export abstract class AbstractContextProvider implements ContextProvider {
  protected cacheDuration: number = 5000; // 5 seconds default cache
  protected cache: Map<string, { value: any; timestamp: number }> = new Map();
  protected maxCacheSize: number = 1000; // Maximum cache entries

  protected getCached<T>(key: string): T | null {
    const cached = this.cache.get(key);
    if (cached && Date.now() - cached.timestamp < this.cacheDuration) {
      // Move to end for LRU behavior
      this.cache.delete(key);
      this.cache.set(key, cached);
      return cached.value as T;
    }
    // Remove expired entry
    if (cached) {
      this.cache.delete(key);
    }
    return null;
  }

  protected setCache(key: string, value: any): void {
    // Remove oldest entries if cache is full
    if (this.cache.size >= this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      if (firstKey) {
        this.cache.delete(firstKey);
      }
    }
    
    // Delete and re-add to move to end (LRU)
    this.cache.delete(key);
    this.cache.set(key, { value, timestamp: Date.now() });
  }

  abstract getLatestPlatformBlockHeight(): Promise<number>;
  abstract getLatestPlatformBlockTime(): Promise<number>;
  abstract getLatestPlatformCoreChainLockedHeight(): Promise<number>;
  abstract getLatestPlatformVersion(): Promise<string>;
  abstract getProposerBlockCount(proposerProTxHash: string): Promise<number | null>;
  abstract getTimePerBlockMillis(): Promise<number>;
  abstract getBlockProposer(blockHeight: number): Promise<string | null>;
  
  async isValid(): Promise<boolean> {
    try {
      const height = await this.getLatestPlatformBlockHeight();
      return height > 0;
    } catch {
      return false;
    }
  }
}