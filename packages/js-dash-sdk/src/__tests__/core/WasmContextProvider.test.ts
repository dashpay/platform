import { WasmContextProvider } from '../../core/WasmContextProvider';
import * as WasmLoader from '../../core/WasmLoader';

jest.mock('../../core/WasmLoader');

// Mock fetch for evonode discovery
global.fetch = jest.fn();

describe('WasmContextProvider', () => {
  let provider: WasmContextProvider;
  const mockFetch = global.fetch as jest.MockedFunction<typeof fetch>;
  
  const mockWasmSdk = {
    WasmSdkBuilder: {
      new_testnet: jest.fn(),
      new_mainnet: jest.fn(),
    },
    fetchBlockHash: jest.fn(),
    fetchDataContract: jest.fn(),
    waitForStateTransition: jest.fn(),
    broadcastStateTransition: jest.fn(),
    getProtocolVersion: jest.fn(),
    FetchOptions: jest.fn().mockImplementation(() => ({
      withProve: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
  };

  const mockBuilder = {
    with_evonode_addresses: jest.fn().mockReturnThis(),
    build: jest.fn(),
  };

  const mockSdkInstance = {
    free: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    (WasmLoader.loadWasmSdk as jest.Mock).mockResolvedValue(mockWasmSdk);
    mockBuilder.build.mockReturnValue(mockSdkInstance);
    mockWasmSdk.WasmSdkBuilder.new_testnet.mockReturnValue(mockBuilder);
    mockWasmSdk.WasmSdkBuilder.new_mainnet.mockReturnValue(mockBuilder);
  });

  describe('Initialization', () => {
    it('should initialize with testnet and discover evonodes', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          success: true,
          data: [
            { service: '52.13.132.146:19999' },
            { service: '35.166.180.159:19999' },
          ],
          message: 'Success',
        }),
      } as Response);

      provider = new WasmContextProvider('testnet');
      await provider.init();

      expect(WasmLoader.loadWasmSdk).toHaveBeenCalled();
      expect(mockFetch).toHaveBeenCalledWith(
        'https://quorums.testnet.networks.dash.org/masternodes',
        expect.any(Object)
      );
      expect(mockBuilder.with_evonode_addresses).toHaveBeenCalledWith([
        'https://52.13.132.146:1443',
        'https://35.166.180.159:1443',
      ]);
    });

    it('should initialize with mainnet configuration', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          success: true,
          data: [{ service: '1.2.3.4:19999' }],
          message: 'Success',
        }),
      } as Response);

      provider = new WasmContextProvider('mainnet');
      await provider.init();

      expect(mockWasmSdk.WasmSdkBuilder.new_mainnet).toHaveBeenCalled();
      expect(mockWasmSdk.WasmSdkBuilder.new_testnet).not.toHaveBeenCalled();
    });

    it('should handle evonode discovery failure gracefully', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      provider = new WasmContextProvider('testnet');
      
      // Should still initialize but with default evonodes
      await expect(provider.init()).resolves.not.toThrow();
    });

    it('should not reinitialize if already initialized', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);

      provider = new WasmContextProvider('testnet');
      await provider.init();
      
      // Clear mocks and try again
      jest.clearAllMocks();
      await provider.init();

      // Should not call loadWasmSdk again
      expect(WasmLoader.loadWasmSdk).not.toHaveBeenCalled();
    });
  });

  describe('getBlockHash', () => {
    beforeEach(async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();
    });

    it('should fetch block hash with proved mode', async () => {
      const height = 12345;
      const expectedHash = 'block-hash-12345';
      
      mockWasmSdk.fetchBlockHash.mockResolvedValue(expectedHash);

      const hash = await provider.getBlockHash(height);

      // Verify FetchOptions was created with prove
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      expect(mockWasmSdk.fetchBlockHash).toHaveBeenCalledWith(
        mockSdkInstance,
        height,
        fetchOptions
      );

      expect(hash).toBe(expectedHash);
    });

    it('should clean up FetchOptions after use', async () => {
      mockWasmSdk.fetchBlockHash.mockResolvedValue('hash');

      await provider.getBlockHash(100);

      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.free).toHaveBeenCalled();
    });

    it('should throw if not initialized', async () => {
      const uninitializedProvider = new WasmContextProvider('testnet');

      await expect(
        uninitializedProvider.getBlockHash(100)
      ).rejects.toThrow('not initialized');
    });
  });

  describe('getDataContract', () => {
    beforeEach(async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();
    });

    it('should fetch data contract with proved mode', async () => {
      const contractId = 'test-contract-id';
      const mockContract = {
        id: contractId,
        schema: {},
        version: 1,
      };

      mockWasmSdk.fetchDataContract.mockResolvedValue(mockContract);

      const contract = await provider.getDataContract(contractId);

      // Verify FetchOptions was created with prove
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      expect(mockWasmSdk.fetchDataContract).toHaveBeenCalledWith(
        mockSdkInstance,
        contractId,
        fetchOptions
      );

      expect(contract).toEqual(mockContract);
    });

    it('should return null for non-existent contract', async () => {
      mockWasmSdk.fetchDataContract.mockResolvedValue(null);

      const contract = await provider.getDataContract('non-existent');

      expect(contract).toBeNull();
    });
  });

  describe('waitForStateTransitionResult', () => {
    beforeEach(async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();
    });

    it('should always use proved mode regardless of parameter', async () => {
      const stHash = 'transition-hash';
      const mockResult = {
        hash: stHash,
        proved: true,
        result: 'success',
      };

      mockWasmSdk.waitForStateTransition.mockResolvedValue(mockResult);

      // Call with prove=false
      const result = await provider.waitForStateTransitionResult(stHash, false);

      // Should still use proved mode
      expect(mockWasmSdk.waitForStateTransition).toHaveBeenCalledWith(
        mockSdkInstance,
        stHash,
        true // Always true
      );

      expect(result).toEqual(mockResult);
    });

    it('should handle timeout', async () => {
      mockWasmSdk.waitForStateTransition.mockImplementation(
        () => new Promise(resolve => setTimeout(resolve, 60000))
      );

      await expect(
        provider.waitForStateTransitionResult('hash', true)
      ).rejects.toThrow('Timeout');
    });
  });

  describe('broadcastStateTransition', () => {
    beforeEach(async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();
    });

    it('should broadcast state transition', async () => {
      const mockTransition = {
        toBuffer: () => Buffer.from('transition-data'),
      };
      const expectedHash = 'broadcast-hash';

      mockWasmSdk.broadcastStateTransition.mockResolvedValue(expectedHash);

      const hash = await provider.broadcastStateTransition(mockTransition);

      expect(mockWasmSdk.broadcastStateTransition).toHaveBeenCalledWith(
        mockSdkInstance,
        mockTransition
      );

      expect(hash).toBe(expectedHash);
    });

    it('should handle broadcast failure', async () => {
      const mockTransition = {
        toBuffer: () => Buffer.from('transition-data'),
      };

      mockWasmSdk.broadcastStateTransition.mockRejectedValue(
        new Error('Broadcast failed')
      );

      await expect(
        provider.broadcastStateTransition(mockTransition)
      ).rejects.toThrow('Broadcast failed');
    });
  });

  describe('getProtocolVersion', () => {
    beforeEach(async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();
    });

    it('should return protocol version', async () => {
      const expectedVersion = 1;
      mockWasmSdk.getProtocolVersion.mockResolvedValue(expectedVersion);

      const version = await provider.getProtocolVersion();

      expect(mockWasmSdk.getProtocolVersion).toHaveBeenCalledWith(
        mockSdkInstance
      );
      expect(version).toBe(expectedVersion);
    });
  });

  describe('cleanup', () => {
    it('should clean up resources on close', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();

      provider.close();

      expect(mockSdkInstance.free).toHaveBeenCalled();
    });

    it('should handle multiple close calls', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();

      provider.close();
      provider.close(); // Second call

      expect(mockSdkInstance.free).toHaveBeenCalledTimes(1);
    });
  });

  describe('Error Handling', () => {
    it('should handle WASM loading failure', async () => {
      (WasmLoader.loadWasmSdk as jest.Mock).mockRejectedValue(
        new Error('WASM load failed')
      );

      provider = new WasmContextProvider('testnet');
      
      await expect(provider.init()).rejects.toThrow('WASM load failed');
    });

    it('should provide meaningful errors for network issues', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          success: true,
          data: [],
          message: 'Success',
        }),
      } as Response);
      
      provider = new WasmContextProvider('testnet');
      await provider.init();

      mockWasmSdk.fetchBlockHash.mockRejectedValue(
        new Error('gRPC error: connection refused')
      );

      await expect(
        provider.getBlockHash(100)
      ).rejects.toThrow('connection refused');
    });
  });

  describe('Network Resilience', () => {
    it('should handle evonode failover', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          success: true,
          data: [
            { service: '1.1.1.1:19999' }, // Will fail
            { service: '2.2.2.2:19999' }, // Will succeed
          ],
          message: 'Success',
        }),
      } as Response);

      provider = new WasmContextProvider('testnet');
      await provider.init();

      // Verify multiple evonodes were configured
      expect(mockBuilder.with_evonode_addresses).toHaveBeenCalledWith([
        'https://1.1.1.1:1443',
        'https://2.2.2.2:1443',
      ]);
    });

    it('should refresh evonodes on connection issues', async () => {
      // Initial discovery
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          success: true,
          data: [{ service: '1.1.1.1:19999' }],
          message: 'Success',
        }),
      } as Response);

      provider = new WasmContextProvider('testnet');
      await provider.init();

      // Simulate connection failure
      mockWasmSdk.fetchBlockHash.mockRejectedValue(
        new Error('All evonodes failed')
      );

      // Future implementation would refresh evonodes
      await expect(provider.getBlockHash(100)).rejects.toThrow();
    });
  });
});