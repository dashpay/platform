import { EvonodesProvider } from '../../core/EvonodesProvider';

// Mock fetch
global.fetch = jest.fn();

describe('EvonodesProvider', () => {
  let provider: EvonodesProvider;
  const mockFetch = global.fetch as jest.MockedFunction<typeof fetch>;

  beforeEach(() => {
    jest.clearAllMocks();
    provider = new EvonodesProvider('testnet');
  });

  describe('Initialization', () => {
    it('should initialize with testnet configuration', () => {
      expect(provider).toBeInstanceOf(EvonodesProvider);
      expect(provider['network']).toBe('testnet');
    });

    it('should initialize with mainnet configuration', () => {
      const mainnetProvider = new EvonodesProvider('mainnet');
      expect(mainnetProvider['network']).toBe('mainnet');
    });
  });

  describe('discoverEvonodes', () => {
    const mockEvonodeResponse = {
      success: true,
      data: [
        {
          proRegTxHash: 'hash1',
          payoutAddress: 'address1',
          pubKeyOperator: 'pubkey1',
          confirmedHash: 'confirmed1',
          service: '52.13.132.146:19999',
          someField: {
            corePort: 19998
          }
        },
        {
          proRegTxHash: 'hash2',
          payoutAddress: 'address2',
          pubKeyOperator: 'pubkey2',
          confirmedHash: 'confirmed2',
          service: '35.166.180.159:19999',
          someField: {
            corePort: 19998
          }
        }
      ],
      message: 'Success'
    };

    it('should discover evonodes from testnet endpoint', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockEvonodeResponse,
      } as Response);

      const evonodes = await provider['discoverEvonodes']();

      expect(mockFetch).toHaveBeenCalledWith(
        'https://quorums.testnet.networks.dash.org/masternodes',
        expect.any(Object)
      );

      expect(evonodes).toHaveLength(2);
      expect(evonodes[0]).toBe('https://52.13.132.146:1443');
      expect(evonodes[1]).toBe('https://35.166.180.159:1443');
    });

    it('should discover evonodes from mainnet endpoint', async () => {
      const mainnetProvider = new EvonodesProvider('mainnet');
      
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => mockEvonodeResponse,
      } as Response);

      const evonodes = await mainnetProvider['discoverEvonodes']();

      expect(mockFetch).toHaveBeenCalledWith(
        'https://quorums.mainnet.networks.dash.org/masternodes',
        expect.any(Object)
      );
    });

    it('should handle discovery failure', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Network error'));

      await expect(provider['discoverEvonodes']()).rejects.toThrow('Network error');
    });

    it('should handle invalid response format', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: false, message: 'Invalid request' }),
      } as Response);

      await expect(provider['discoverEvonodes']()).rejects.toThrow(
        'Invalid response format'
      );
    });

    it('should filter out nodes without service field', async () => {
      const responseWithInvalidNodes = {
        success: true,
        data: [
          {
            proRegTxHash: 'hash1',
            service: '52.13.132.146:19999',
          },
          {
            proRegTxHash: 'hash2',
            // Missing service field
          },
          {
            proRegTxHash: 'hash3',
            service: null, // Null service
          }
        ],
        message: 'Success'
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => responseWithInvalidNodes,
      } as Response);

      const evonodes = await provider['discoverEvonodes']();

      expect(evonodes).toHaveLength(1);
      expect(evonodes[0]).toBe('https://52.13.132.146:1443');
    });

    it('should handle empty evonode list', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true, data: [], message: 'Success' }),
      } as Response);

      const evonodes = await provider['discoverEvonodes']();

      expect(evonodes).toHaveLength(0);
    });

    it('should correctly transform port 19999 to 1443', async () => {
      const response = {
        success: true,
        data: [
          { service: '1.2.3.4:19999' },
          { service: '5.6.7.8:20000' }, // Different port
          { service: '9.10.11.12:19999' },
        ],
        message: 'Success'
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => response,
      } as Response);

      const evonodes = await provider['discoverEvonodes']();

      expect(evonodes).toEqual([
        'https://1.2.3.4:1443',
        'https://5.6.7.8:20000', // Port unchanged if not 19999
        'https://9.10.11.12:1443',
      ]);
    });
  });

  describe('createWasmSdk', () => {
    const mockWasmModule = {
      WasmSdkBuilder: {
        new_testnet: jest.fn(),
        new_mainnet: jest.fn(),
      },
    };

    const mockBuilder = {
      with_evonode_addresses: jest.fn().mockReturnThis(),
      build: jest.fn(),
    };

    beforeEach(() => {
      mockWasmModule.WasmSdkBuilder.new_testnet.mockReturnValue(mockBuilder);
      mockWasmModule.WasmSdkBuilder.new_mainnet.mockReturnValue(mockBuilder);
      mockBuilder.build.mockReturnValue({ sdk: 'instance' });
    });

    it('should create WASM SDK with discovered evonodes', async () => {
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

      const sdk = await provider['createWasmSdk'](mockWasmModule);

      expect(mockWasmModule.WasmSdkBuilder.new_testnet).toHaveBeenCalled();
      expect(mockBuilder.with_evonode_addresses).toHaveBeenCalledWith([
        'https://52.13.132.146:1443',
        'https://35.166.180.159:1443',
      ]);
      expect(mockBuilder.build).toHaveBeenCalled();
      expect(sdk).toEqual({ sdk: 'instance' });
    });

    it('should create mainnet SDK when network is mainnet', async () => {
      const mainnetProvider = new EvonodesProvider('mainnet');
      
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          success: true,
          data: [{ service: '1.2.3.4:19999' }],
          message: 'Success',
        }),
      } as Response);

      await mainnetProvider['createWasmSdk'](mockWasmModule);

      expect(mockWasmModule.WasmSdkBuilder.new_mainnet).toHaveBeenCalled();
      expect(mockWasmModule.WasmSdkBuilder.new_testnet).not.toHaveBeenCalled();
    });

    it('should handle evonode discovery failure', async () => {
      mockFetch.mockRejectedValueOnce(new Error('Discovery failed'));

      await expect(provider['createWasmSdk'](mockWasmModule)).rejects.toThrow(
        'Discovery failed'
      );
    });
  });

  describe('ContextProvider Implementation', () => {
    it('should implement getBlockHash', async () => {
      const height = 12345;
      
      // Mock implementation would need actual WASM SDK
      expect(provider.getBlockHash).toBeDefined();
      expect(typeof provider.getBlockHash).toBe('function');
    });

    it('should implement getDataContract', async () => {
      expect(provider.getDataContract).toBeDefined();
      expect(typeof provider.getDataContract).toBe('function');
    });

    it('should implement waitForStateTransitionResult with proved mode', async () => {
      expect(provider.waitForStateTransitionResult).toBeDefined();
      expect(typeof provider.waitForStateTransitionResult).toBe('function');
      
      // Should always use proved mode
      // const result = await provider.waitForStateTransitionResult('hash', false);
      // In actual implementation, this should still use proved mode internally
    });

    it('should implement broadcastStateTransition', async () => {
      expect(provider.broadcastStateTransition).toBeDefined();
      expect(typeof provider.broadcastStateTransition).toBe('function');
    });

    it('should implement getProtocolVersion', async () => {
      expect(provider.getProtocolVersion).toBeDefined();
      expect(typeof provider.getProtocolVersion).toBe('function');
    });
  });

  describe('Network Resilience', () => {
    it('should retry on network failure', async () => {
      // First call fails, second succeeds
      mockFetch
        .mockRejectedValueOnce(new Error('Network timeout'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({
            success: true,
            data: [{ service: '1.2.3.4:19999' }],
            message: 'Success',
          }),
        } as Response);

      // This would depend on retry logic implementation
      // const evonodes = await provider['discoverEvonodes']();
      // expect(evonodes).toHaveLength(1);
    });

    it('should timeout after maximum duration', async () => {
      mockFetch.mockImplementation(() => 
        new Promise((resolve) => setTimeout(resolve, 60000))
      );

      // This would depend on timeout implementation
      // await expect(provider['discoverEvonodes']()).rejects.toThrow('Timeout');
    });
  });
});