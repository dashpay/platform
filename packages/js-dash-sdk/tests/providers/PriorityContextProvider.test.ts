import { PriorityContextProvider } from '../../src/providers/PriorityContextProvider';
import { ContextProvider } from '../../src/core/types';
import { ProviderCapability } from '../../src/providers/types';

// Mock provider implementation
class MockProvider implements ContextProvider {
  constructor(
    private name: string,
    private shouldFail: boolean = false,
    private responseDelay: number = 10
  ) {}

  async getLatestPlatformBlockHeight(): Promise<number> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return 123456;
  }

  async getLatestPlatformBlockTime(): Promise<number> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return Date.now();
  }

  async getLatestPlatformCoreChainLockedHeight(): Promise<number> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return 123400;
  }

  async getLatestPlatformVersion(): Promise<string> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return '1.0.0';
  }

  async getProposerBlockCount(proposerProTxHash: string): Promise<number | null> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return 42;
  }

  async getTimePerBlockMillis(): Promise<number> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return 2500;
  }

  async getBlockProposer(blockHeight: number): Promise<string | null> {
    await this.delay();
    if (this.shouldFail) throw new Error(`${this.name} failed`);
    return 'proposer123';
  }

  async isValid(): Promise<boolean> {
    return !this.shouldFail;
  }

  private delay(): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, this.responseDelay));
  }
}

describe('PriorityContextProvider', () => {
  describe('basic functionality', () => {
    it('should use highest priority provider when available', async () => {
      const provider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('Low', false, 50),
            priority: 10,
            name: 'Low'
          },
          {
            provider: new MockProvider('High', false, 10),
            priority: 100,
            name: 'High'
          },
          {
            provider: new MockProvider('Medium', false, 30),
            priority: 50,
            name: 'Medium'
          }
        ]
      });

      const start = Date.now();
      const height = await provider.getLatestPlatformBlockHeight();
      const duration = Date.now() - start;

      expect(height).toBe(123456);
      // Should use High provider (10ms delay)
      expect(duration).toBeLessThan(25);
    });

    it('should fallback to next provider on failure', async () => {
      const events: string[] = [];

      const provider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('Primary', true), // Will fail
            priority: 100,
            name: 'Primary'
          },
          {
            provider: new MockProvider('Secondary', false),
            priority: 50,
            name: 'Secondary'
          }
        ],
        fallbackEnabled: true
      });

      provider.on('provider:error', (name) => events.push(`error:${name}`));
      provider.on('provider:fallback', (from, to) => events.push(`fallback:${from}->${to}`));
      provider.on('provider:used', (name) => events.push(`used:${name}`));

      const height = await provider.getLatestPlatformBlockHeight();

      expect(height).toBe(123456);
      expect(events).toContain('error:Primary');
      expect(events).toContain('used:Secondary');
    });

    it('should throw when all providers fail', async () => {
      const provider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('Provider1', true),
            priority: 100,
            name: 'Provider1'
          },
          {
            provider: new MockProvider('Provider2', true),
            priority: 50,
            name: 'Provider2'
          }
        ]
      });

      await expect(provider.getLatestPlatformBlockHeight())
        .rejects.toThrow('All providers failed');
    });
  });

  describe('caching', () => {
    it('should cache results when enabled', async () => {
      let callCount = 0;
      const mockProvider = new MockProvider('Test', false);
      const originalMethod = mockProvider.getLatestPlatformBlockHeight;
      mockProvider.getLatestPlatformBlockHeight = async () => {
        callCount++;
        return originalMethod.call(mockProvider);
      };

      const provider = new PriorityContextProvider({
        providers: [{
          provider: mockProvider,
          priority: 100,
          name: 'Test'
        }],
        cacheResults: true
      });

      // First call
      await provider.getLatestPlatformBlockHeight();
      expect(callCount).toBe(1);

      // Second call should use cache
      await provider.getLatestPlatformBlockHeight();
      expect(callCount).toBe(1);

      // Clear cache and call again
      provider.clearCache();
      await provider.getLatestPlatformBlockHeight();
      expect(callCount).toBe(2);
    });
  });

  describe('metrics', () => {
    it('should track provider metrics', async () => {
      const provider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('Success', false, 10),
            priority: 100,
            name: 'Success'
          },
          {
            provider: new MockProvider('Failure', true),
            priority: 50,
            name: 'Failure'
          }
        ]
      });

      // Make some successful calls
      await provider.getLatestPlatformBlockHeight();
      await provider.getLatestPlatformBlockTime();

      // Make a call that will fail on first provider
      const failProvider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('WillFail', true),
            priority: 100,
            name: 'WillFail'
          },
          {
            provider: new MockProvider('WillSucceed', false),
            priority: 50,
            name: 'WillSucceed'
          }
        ]
      });

      await failProvider.getLatestPlatformVersion();

      const metrics = provider.getMetrics();
      const successMetrics = metrics.get('Success');

      expect(successMetrics).toBeDefined();
      expect(successMetrics!.successCount).toBe(2);
      expect(successMetrics!.errorCount).toBe(0);
      expect(successMetrics!.averageResponseTime).toBeGreaterThan(0);
    });
  });

  describe('provider management', () => {
    it('should add and remove providers dynamically', async () => {
      const provider = new PriorityContextProvider({
        providers: [{
          provider: new MockProvider('Initial'),
          priority: 50,
          name: 'Initial'
        }]
      });

      // Add a higher priority provider
      provider.addProvider(
        new MockProvider('HighPriority'),
        100,
        'HighPriority'
      );

      const activeProvider = await provider.getActiveProvider();
      expect(activeProvider?.name).toBe('HighPriority');

      // Remove the high priority provider
      provider.removeProvider('HighPriority');

      const newActiveProvider = await provider.getActiveProvider();
      expect(newActiveProvider?.name).toBe('Initial');
    });
  });

  describe('events', () => {
    it('should emit all expected events', async () => {
      const events: any[] = [];

      const provider = new PriorityContextProvider({
        providers: [
          {
            provider: new MockProvider('Primary', true),
            priority: 100,
            name: 'Primary'
          },
          {
            provider: new MockProvider('Secondary', true),
            priority: 50,
            name: 'Secondary'
          }
        ],
        logErrors: true
      });

      provider.on('provider:error', (name, error) => {
        events.push({ type: 'error', name, error: error.message });
      });

      provider.on('all:failed', (method, errors) => {
        events.push({ type: 'all:failed', method, errorCount: errors.size });
      });

      try {
        await provider.getLatestPlatformBlockHeight();
      } catch {
        // Expected to fail
      }

      expect(events).toHaveLength(3); // 2 errors + 1 all:failed
      expect(events.find(e => e.type === 'all:failed')).toBeDefined();
      expect(events.filter(e => e.type === 'error')).toHaveLength(2);
    });
  });
});