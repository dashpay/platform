import { WebServiceProvider } from '../../src/providers/WebServiceProvider';
import { ProviderCapability } from '../../src/providers/types';

// Mock fetch for testing
global.fetch = jest.fn();

describe('WebServiceProvider', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with testnet by default', () => {
      const provider = new WebServiceProvider();
      expect(provider.getName()).toBe('WebServiceProvider');
      expect(provider.getCapabilities()).toContain(ProviderCapability.PLATFORM_STATE);
      expect(provider.getCapabilities()).toContain(ProviderCapability.QUORUM_KEYS);
    });

    it('should initialize with custom URL', () => {
      const provider = new WebServiceProvider({
        url: 'https://custom.quorum.service'
      });
      expect(provider.getName()).toBe('WebServiceProvider');
    });

    it('should set mainnet URL when specified', () => {
      const provider = new WebServiceProvider({
        network: 'mainnet'
      });
      expect(provider.getName()).toBe('WebServiceProvider');
    });
  });

  describe('platform state methods', () => {
    it('should fetch platform block height', async () => {
      const mockResponse = {
        platform: {
          blockHeight: 123456
        }
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse
      });

      const provider = new WebServiceProvider();
      const height = await provider.getLatestPlatformBlockHeight();

      expect(height).toBe(123456);
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/status'),
        expect.any(Object)
      );
    });

    it('should handle alternative response format', async () => {
      const mockResponse = {
        blockHeight: 123456 // Direct property
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse
      });

      const provider = new WebServiceProvider();
      const height = await provider.getLatestPlatformBlockHeight();

      expect(height).toBe(123456);
    });

    it('should cache responses', async () => {
      const mockResponse = {
        platform: {
          blockHeight: 123456
        }
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse
      });

      const provider = new WebServiceProvider({ cacheDuration: 1000 });
      
      // First call
      await provider.getLatestPlatformBlockHeight();
      expect(global.fetch).toHaveBeenCalledTimes(1);

      // Second call should use cache
      await provider.getLatestPlatformBlockHeight();
      expect(global.fetch).toHaveBeenCalledTimes(1);
    });
  });

  describe('quorum operations', () => {
    it('should fetch quorum keys', async () => {
      const mockQuorums = {
        'quorum1': {
          publicKey: 'pubkey1',
          version: 1,
          type: 'BLS'
        },
        'quorum2': {
          publicKey: 'pubkey2',
          version: 2,
          type: 'ECDSA'
        }
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockQuorums
      });

      const provider = new WebServiceProvider();
      const quorums = await provider.getQuorumKeys();

      expect(quorums.size).toBe(2);
      expect(quorums.get('quorum1')).toBeDefined();
      expect(quorums.get('quorum1')?.quorumPublicKey.publicKey).toBe('pubkey1');
      expect(quorums.get('quorum1')?.quorumPublicKey.type).toBe('BLS');
    });

    it('should get specific quorum', async () => {
      const mockQuorums = {
        'quorum1': {
          publicKey: 'pubkey1',
          version: 1,
          type: 'BLS'
        }
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockQuorums
      });

      const provider = new WebServiceProvider();
      const quorum = await provider.getQuorum('quorum1');

      expect(quorum).toBeDefined();
      expect(quorum?.quorumHash).toBe('quorum1');
    });

    it('should get active quorums', async () => {
      const mockQuorums = {
        'quorum1': {
          publicKey: 'pubkey1',
          version: 1,
          type: 'BLS'
        },
        'quorum2': {
          publicKey: 'pubkey2',
          version: 2,
          type: 'BLS'
        }
      };

      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => mockQuorums
      });

      const provider = new WebServiceProvider();
      const activeQuorums = await provider.getActiveQuorums();

      expect(activeQuorums).toHaveLength(2);
      expect(activeQuorums[0].isActive).toBe(true);
    });
  });

  describe('retry logic', () => {
    it('should retry on network failure', async () => {
      const mockResponse = {
        platform: {
          blockHeight: 123456
        }
      };

      // First call fails, second succeeds
      (global.fetch as jest.Mock)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => mockResponse
        });

      const provider = new WebServiceProvider({
        retryAttempts: 1,
        retryDelay: 10
      });

      const height = await provider.getLatestPlatformBlockHeight();

      expect(height).toBe(123456);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });

    it('should not retry on client errors', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request'
      });

      const provider = new WebServiceProvider({
        retryAttempts: 3
      });

      await expect(provider.getLatestPlatformBlockHeight())
        .rejects.toThrow('HTTP 400');

      expect(global.fetch).toHaveBeenCalledTimes(1);
    });

    it('should fail after max retries', async () => {
      (global.fetch as jest.Mock).mockRejectedValue(new Error('Network error'));

      const provider = new WebServiceProvider({
        retryAttempts: 2,
        retryDelay: 10
      });

      await expect(provider.getLatestPlatformBlockHeight())
        .rejects.toThrow('Failed after 3 attempts');

      expect(global.fetch).toHaveBeenCalledTimes(3);
    });
  });

  describe('availability check', () => {
    it('should return true when service is available', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        json: async () => ({})
      });

      const provider = new WebServiceProvider();
      const available = await provider.isAvailable();

      expect(available).toBe(true);
    });

    it('should return false when service is not available', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('Connection refused'));

      const provider = new WebServiceProvider();
      const available = await provider.isAvailable();

      expect(available).toBe(false);
    });
  });

  describe('timeout handling', () => {
    it('should timeout long requests', async () => {
      jest.useFakeTimers();
      
      // Mock fetch that resolves after timeout
      let fetchResolve: (value: any) => void;
      const fetchPromise = new Promise((resolve) => {
        fetchResolve = resolve;
      });
      
      (global.fetch as jest.Mock).mockImplementationOnce(() => fetchPromise);

      const provider = new WebServiceProvider({
        timeout: 100 // 100ms timeout
      });

      // Start the request
      const requestPromise = provider.getLatestPlatformBlockHeight();
      
      // Advance timers past timeout
      jest.advanceTimersByTime(150);
      
      // Should reject with timeout error
      await expect(requestPromise).rejects.toThrow('aborted');
      
      // Clean up - resolve the fetch to avoid hanging promise
      fetchResolve!({
        ok: true,
        json: async () => ({ platform: { blockHeight: 123 } })
      });
      
      jest.useRealTimers();
    });
  });
});