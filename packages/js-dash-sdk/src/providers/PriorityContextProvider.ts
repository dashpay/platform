/**
 * Priority-based context provider that tries multiple providers in order
 */

import { EventEmitter } from 'eventemitter3';
import { ContextProvider } from '../core/types';
import { 
  PriorityProviderOptions,
  PriorityProviderEvents,
  ProviderMetrics,
  ProviderWithCapabilities,
  ProviderCapability
} from './types';

interface ProviderEntry {
  provider: ContextProvider;
  priority: number;
  name: string;
  capabilities: ProviderCapability[];
  metrics: ProviderMetrics;
}

export class PriorityContextProvider extends EventEmitter<PriorityProviderEvents> implements ContextProvider {
  private providers: ProviderEntry[] = [];
  private fallbackEnabled: boolean;
  private cacheResults: boolean;
  private logErrors: boolean;
  private cache = new Map<string, { value: any; timestamp: number }>();
  private cacheDuration = 5000; // 5 seconds
  
  constructor(options: PriorityProviderOptions) {
    super();
    
    this.fallbackEnabled = options.fallbackEnabled ?? true;
    this.cacheResults = options.cacheResults ?? true;
    this.logErrors = options.logErrors ?? false;
    
    // Sort providers by priority (higher number = higher priority)
    const sortedProviders = [...options.providers].sort((a, b) => b.priority - a.priority);
    
    // Initialize provider entries
    this.providers = sortedProviders.map(({ provider, priority, name, capabilities }) => ({
      provider,
      priority,
      name: name || this.getProviderName(provider),
      capabilities: capabilities || this.getProviderCapabilities(provider),
      metrics: {
        successCount: 0,
        errorCount: 0,
        averageResponseTime: 0,
      },
    }));
  }
  
  /**
   * Execute a method on providers in priority order
   */
  private async executeWithPriority<T>(
    method: string,
    executor: (provider: ContextProvider) => Promise<T>,
    requiredCapability?: ProviderCapability
  ): Promise<T> {
    // Check cache first
    if (this.cacheResults) {
      const cached = this.getCached<T>(method);
      if (cached !== null) return cached;
    }
    
    const errors = new Map<string, Error>();
    let lastProvider: ProviderEntry | null = null;
    
    // Filter providers by capability if required
    const eligibleProviders = requiredCapability
      ? this.providers.filter(p => p.capabilities.includes(requiredCapability))
      : this.providers;
    
    for (const providerEntry of eligibleProviders) {
      const startTime = Date.now();
      
      try {
        // Check if provider is available
        if ('isAvailable' in providerEntry.provider) {
          const available = await (providerEntry.provider as ProviderWithCapabilities).isAvailable();
          if (!available) {
            throw new Error('Provider not available');
          }
        }
        
        // Execute the method
        const result = await executor(providerEntry.provider);
        
        // Update metrics
        const responseTime = Date.now() - startTime;
        this.updateMetrics(providerEntry, true, responseTime);
        
        // Cache result
        if (this.cacheResults) {
          this.setCache(method, result);
        }
        
        // Emit success event
        this.emit('provider:used', providerEntry.name, method);
        
        return result;
      } catch (error: any) {
        // Update metrics
        this.updateMetrics(providerEntry, false, Date.now() - startTime, error);
        errors.set(providerEntry.name, error);
        
        // Log error if enabled
        if (this.logErrors) {
          console.error(`Provider ${providerEntry.name} failed for ${method}:`, error.message);
        }
        
        // Emit error event
        this.emit('provider:error', providerEntry.name, error);
        
        // Try fallback if enabled
        if (this.fallbackEnabled && lastProvider) {
          this.emit('provider:fallback', lastProvider.name, providerEntry.name);
        }
        
        lastProvider = providerEntry;
      }
    }
    
    // All providers failed
    this.emit('all:failed', method, errors);
    
    const errorMessages = Array.from(errors.entries())
      .map(([name, error]) => `${name}: ${error.message}`)
      .join(', ');
    
    throw new Error(`All providers failed for ${method}: ${errorMessages}`);
  }
  
  async getLatestPlatformBlockHeight(): Promise<number> {
    return this.executeWithPriority(
      'getLatestPlatformBlockHeight',
      provider => provider.getLatestPlatformBlockHeight(),
      ProviderCapability.PLATFORM_STATE
    );
  }
  
  async getLatestPlatformBlockTime(): Promise<number> {
    return this.executeWithPriority(
      'getLatestPlatformBlockTime',
      provider => provider.getLatestPlatformBlockTime(),
      ProviderCapability.PLATFORM_STATE
    );
  }
  
  async getLatestPlatformCoreChainLockedHeight(): Promise<number> {
    return this.executeWithPriority(
      'getLatestPlatformCoreChainLockedHeight',
      provider => provider.getLatestPlatformCoreChainLockedHeight(),
      ProviderCapability.PLATFORM_STATE
    );
  }
  
  async getLatestPlatformVersion(): Promise<string> {
    return this.executeWithPriority(
      'getLatestPlatformVersion',
      provider => provider.getLatestPlatformVersion(),
      ProviderCapability.PLATFORM_STATE
    );
  }
  
  async getProposerBlockCount(proposerProTxHash: string): Promise<number | null> {
    return this.executeWithPriority(
      `getProposerBlockCount:${proposerProTxHash}`,
      provider => provider.getProposerBlockCount(proposerProTxHash),
      ProviderCapability.BLOCK_PROPOSER
    );
  }
  
  async getTimePerBlockMillis(): Promise<number> {
    return this.executeWithPriority(
      'getTimePerBlockMillis',
      provider => provider.getTimePerBlockMillis(),
      ProviderCapability.PLATFORM_STATE
    );
  }
  
  async getBlockProposer(blockHeight: number): Promise<string | null> {
    return this.executeWithPriority(
      `getBlockProposer:${blockHeight}`,
      provider => provider.getBlockProposer(blockHeight),
      ProviderCapability.BLOCK_PROPOSER
    );
  }
  
  async isValid(): Promise<boolean> {
    try {
      await this.getLatestPlatformBlockHeight();
      return true;
    } catch {
      return false;
    }
  }
  
  /**
   * Get metrics for all providers
   */
  getMetrics(): Map<string, ProviderMetrics> {
    const metrics = new Map<string, ProviderMetrics>();
    
    for (const provider of this.providers) {
      metrics.set(provider.name, { ...provider.metrics });
    }
    
    return metrics;
  }
  
  /**
   * Get the currently active provider (highest priority available)
   */
  async getActiveProvider(): Promise<ProviderEntry | null> {
    for (const provider of this.providers) {
      try {
        if ('isAvailable' in provider.provider) {
          const available = await (provider.provider as ProviderWithCapabilities).isAvailable();
          if (available) return provider;
        } else {
          // Try a simple operation to check availability
          await provider.provider.getLatestPlatformBlockHeight();
          return provider;
        }
      } catch {
        continue;
      }
    }
    return null;
  }
  
  /**
   * Add a new provider
   */
  addProvider(
    provider: ContextProvider,
    priority: number,
    name?: string,
    capabilities?: ProviderCapability[]
  ): void {
    const entry: ProviderEntry = {
      provider,
      priority,
      name: name || this.getProviderName(provider),
      capabilities: capabilities || this.getProviderCapabilities(provider),
      metrics: {
        successCount: 0,
        errorCount: 0,
        averageResponseTime: 0,
      },
    };
    
    this.providers.push(entry);
    this.providers.sort((a, b) => b.priority - a.priority);
  }
  
  /**
   * Remove a provider by name
   */
  removeProvider(name: string): boolean {
    const index = this.providers.findIndex(p => p.name === name);
    if (index >= 0) {
      this.providers.splice(index, 1);
      return true;
    }
    return false;
  }
  
  /**
   * Clear cache
   */
  clearCache(): void {
    this.cache.clear();
  }
  
  private updateMetrics(
    provider: ProviderEntry,
    success: boolean,
    responseTime: number,
    error?: Error
  ): void {
    if (success) {
      provider.metrics.successCount++;
      provider.metrics.lastSuccessTime = new Date();
      
      // Update average response time
      const totalRequests = provider.metrics.successCount + provider.metrics.errorCount;
      provider.metrics.averageResponseTime = 
        (provider.metrics.averageResponseTime * (totalRequests - 1) + responseTime) / totalRequests;
    } else {
      provider.metrics.errorCount++;
      if (error) {
        provider.metrics.lastError = error;
      }
    }
  }
  
  private getCached<T>(key: string): T | null {
    const cached = this.cache.get(key);
    if (cached && Date.now() - cached.timestamp < this.cacheDuration) {
      return cached.value as T;
    }
    return null;
  }
  
  private setCache(key: string, value: any): void {
    this.cache.set(key, { value, timestamp: Date.now() });
  }
  
  private getProviderName(provider: ContextProvider): string {
    if ('getName' in provider && typeof provider.getName === 'function') {
      return (provider as ProviderWithCapabilities).getName();
    }
    return provider.constructor.name;
  }
  
  private getProviderCapabilities(provider: ContextProvider): ProviderCapability[] {
    if ('getCapabilities' in provider && typeof provider.getCapabilities === 'function') {
      return (provider as ProviderWithCapabilities).getCapabilities();
    }
    // Default capabilities for standard providers
    return [
      ProviderCapability.PLATFORM_STATE,
      ProviderCapability.BLOCK_PROPOSER,
    ];
  }
}