import { ContextProvider } from './types';

export abstract class AbstractContextProvider implements ContextProvider {
  protected cacheDuration: number = 5000; // 5 seconds default cache
  protected cache: Map<string, { value: any; timestamp: number }> = new Map();

  protected getCached<T>(key: string): T | null {
    const cached = this.cache.get(key);
    if (cached && Date.now() - cached.timestamp < this.cacheDuration) {
      return cached.value as T;
    }
    return null;
  }

  protected setCache(key: string, value: any): void {
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