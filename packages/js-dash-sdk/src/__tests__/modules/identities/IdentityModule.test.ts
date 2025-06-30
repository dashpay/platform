import { IdentityModule } from '../../../modules/identities/IdentityModule';
import { SDK } from '../../../SDK';
import * as WasmLoader from '../../../core/WasmLoader';

jest.mock('../../../core/WasmLoader');

describe('IdentityModule', () => {
  let sdk: SDK;
  let identityModule: IdentityModule;
  
  const mockWasmSdk = {
    WasmSdkBuilder: {
      new_testnet: jest.fn().mockReturnValue({
        build: jest.fn().mockReturnValue({
          free: jest.fn(),
        })
      })
    },
    fetchIdentityBalance: jest.fn(),
    fetchIdentity: jest.fn(),
    createIdentity: jest.fn(),
    topUpIdentity: jest.fn(),
    FetchOptions: jest.fn().mockImplementation(() => ({
      withProve: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
    CreateIdentityOptions: jest.fn().mockImplementation(() => ({
      withFunding: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
  };

  // Test identity from the user
  const TEST_IDENTITY_ID = global.TEST_IDENTITY_ID;
  const TEST_PRIVATE_KEY = global.TEST_PRIVATE_KEY;

  beforeEach(async () => {
    jest.clearAllMocks();
    (WasmLoader.loadWasmSdk as jest.Mock).mockResolvedValue(mockWasmSdk);
    
    sdk = new SDK({ network: 'testnet' });
    await sdk.init();
    identityModule = new IdentityModule(sdk);
  });

  afterEach(() => {
    sdk.close();
  });

  describe('getBalance', () => {
    it('should fetch identity balance with proved query', async () => {
      const mockBalance = 1000000; // 0.01 DASH in duffs
      mockWasmSdk.fetchIdentityBalance.mockResolvedValue(mockBalance);

      const balance = await identityModule.getBalance(TEST_IDENTITY_ID);

      // Verify FetchOptions was created and withProve was called
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      // Verify the WASM call
      expect(mockWasmSdk.fetchIdentityBalance).toHaveBeenCalledWith(
        expect.any(Object), // SDK instance
        TEST_IDENTITY_ID,
        fetchOptions
      );

      expect(balance).toBe(mockBalance);
    });

    it('should handle identity not found', async () => {
      mockWasmSdk.fetchIdentityBalance.mockRejectedValue(
        new Error('Identity not found')
      );

      await expect(
        identityModule.getBalance('invalid-identity-id')
      ).rejects.toThrow('Identity not found');
    });

    it('should clean up FetchOptions after use', async () => {
      mockWasmSdk.fetchIdentityBalance.mockResolvedValue(1000000);

      await identityModule.getBalance(TEST_IDENTITY_ID);

      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.free).toHaveBeenCalled();
    });

    it('should handle network errors gracefully', async () => {
      mockWasmSdk.fetchIdentityBalance.mockRejectedValue(
        new Error('Network timeout')
      );

      await expect(
        identityModule.getBalance(TEST_IDENTITY_ID)
      ).rejects.toThrow('Network timeout');
    });
  });

  describe('get', () => {
    const mockIdentityData = {
      id: TEST_IDENTITY_ID,
      balance: 1000000,
      publicKeys: [
        {
          id: 0,
          purpose: 0,
          securityLevel: 0,
          data: 'public-key-data',
        }
      ],
      revision: 1,
    };

    it('should fetch full identity with proved query', async () => {
      mockWasmSdk.fetchIdentity.mockResolvedValue(mockIdentityData);

      const identity = await identityModule.get(TEST_IDENTITY_ID);

      // Verify FetchOptions was created with prove
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      // Verify the WASM call
      expect(mockWasmSdk.fetchIdentity).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_IDENTITY_ID,
        fetchOptions
      );

      expect(identity).toEqual(mockIdentityData);
    });

    it('should return null for non-existent identity', async () => {
      mockWasmSdk.fetchIdentity.mockResolvedValue(null);

      const identity = await identityModule.get('non-existent-id');

      expect(identity).toBeNull();
    });

    it('should validate identity ID format', async () => {
      await expect(
        identityModule.get('invalid-format')
      ).rejects.toThrow();
    });
  });

  describe('create', () => {
    it('should create identity with funding', async () => {
      const mockNewIdentity = {
        id: 'new-identity-id',
        balance: 10000000, // 0.1 DASH funding
        publicKeys: [],
        revision: 0,
      };

      mockWasmSdk.createIdentity.mockResolvedValue(mockNewIdentity);

      const identity = await identityModule.create({
        fundingAmount: 10000000,
        privateKey: TEST_PRIVATE_KEY,
      });

      // Verify CreateIdentityOptions was used
      expect(mockWasmSdk.CreateIdentityOptions).toHaveBeenCalled();
      const createOptions = mockWasmSdk.CreateIdentityOptions.mock.results[0].value;
      expect(createOptions.withFunding).toHaveBeenCalledWith(10000000);

      expect(mockWasmSdk.createIdentity).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_PRIVATE_KEY,
        createOptions
      );

      expect(identity).toEqual(mockNewIdentity);
    });

    it('should validate minimum funding amount', async () => {
      await expect(
        identityModule.create({
          fundingAmount: 100, // Too small
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Insufficient funding amount');
    });

    it('should clean up CreateIdentityOptions after use', async () => {
      mockWasmSdk.createIdentity.mockResolvedValue({ id: 'new-id' });

      await identityModule.create({
        fundingAmount: 10000000,
        privateKey: TEST_PRIVATE_KEY,
      });

      const createOptions = mockWasmSdk.CreateIdentityOptions.mock.results[0].value;
      expect(createOptions.free).toHaveBeenCalled();
    });
  });

  describe('topUp', () => {
    it('should top up identity balance', async () => {
      const topUpAmount = 5000000; // 0.05 DASH
      const mockResult = {
        success: true,
        newBalance: 6000000,
      };

      mockWasmSdk.topUpIdentity.mockResolvedValue(mockResult);

      const result = await identityModule.topUp({
        identityId: TEST_IDENTITY_ID,
        amount: topUpAmount,
        privateKey: TEST_PRIVATE_KEY,
      });

      expect(mockWasmSdk.topUpIdentity).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_IDENTITY_ID,
        topUpAmount,
        TEST_PRIVATE_KEY
      );

      expect(result).toEqual(mockResult);
    });

    it('should validate minimum top-up amount', async () => {
      await expect(
        identityModule.topUp({
          identityId: TEST_IDENTITY_ID,
          amount: 100, // Too small
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Amount too small');
    });

    it('should handle insufficient wallet balance', async () => {
      mockWasmSdk.topUpIdentity.mockRejectedValue(
        new Error('Insufficient balance')
      );

      await expect(
        identityModule.topUp({
          identityId: TEST_IDENTITY_ID,
          amount: 100000000, // 1 DASH
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Insufficient balance');
    });
  });

  describe('listPublicKeys', () => {
    it('should list all public keys for identity', async () => {
      const mockIdentity = {
        id: TEST_IDENTITY_ID,
        publicKeys: [
          {
            id: 0,
            purpose: 0,
            securityLevel: 0,
            data: 'key-0-data',
          },
          {
            id: 1,
            purpose: 0,
            securityLevel: 1,
            data: 'key-1-data',
          },
        ],
      };

      mockWasmSdk.fetchIdentity.mockResolvedValue(mockIdentity);

      const publicKeys = await identityModule.listPublicKeys(TEST_IDENTITY_ID);

      expect(publicKeys).toHaveLength(2);
      expect(publicKeys[0].id).toBe(0);
      expect(publicKeys[1].id).toBe(1);
    });

    it('should return empty array for identity without keys', async () => {
      mockWasmSdk.fetchIdentity.mockResolvedValue({
        id: TEST_IDENTITY_ID,
        publicKeys: [],
      });

      const publicKeys = await identityModule.listPublicKeys(TEST_IDENTITY_ID);

      expect(publicKeys).toEqual([]);
    });
  });

  describe('Error Handling', () => {
    it('should handle WASM SDK not initialized', async () => {
      const uninitializedSdk = new DashSDK({ network: 'testnet' });
      const module = new IdentityModule(uninitializedSdk);

      await expect(
        module.getBalance(TEST_IDENTITY_ID)
      ).rejects.toThrow('SDK not initialized');
    });

    it('should provide meaningful error messages', async () => {
      mockWasmSdk.fetchIdentityBalance.mockRejectedValue(
        new Error('gRPC error: connection refused')
      );

      await expect(
        identityModule.getBalance(TEST_IDENTITY_ID)
      ).rejects.toThrow('connection refused');
    });
  });

  describe('Integration Tests', () => {
    it('should perform multiple operations in sequence', async () => {
      // First check balance
      mockWasmSdk.fetchIdentityBalance.mockResolvedValue(1000000);
      const initialBalance = await identityModule.getBalance(TEST_IDENTITY_ID);
      expect(initialBalance).toBe(1000000);

      // Then top up
      mockWasmSdk.topUpIdentity.mockResolvedValue({
        success: true,
        newBalance: 2000000,
      });
      const topUpResult = await identityModule.topUp({
        identityId: TEST_IDENTITY_ID,
        amount: 1000000,
        privateKey: TEST_PRIVATE_KEY,
      });
      expect(topUpResult.newBalance).toBe(2000000);

      // Verify new balance
      mockWasmSdk.fetchIdentityBalance.mockResolvedValue(2000000);
      const newBalance = await identityModule.getBalance(TEST_IDENTITY_ID);
      expect(newBalance).toBe(2000000);
    });

    it('should always use proved queries in all operations', async () => {
      // Get balance
      mockWasmSdk.fetchIdentityBalance.mockResolvedValue(1000000);
      await identityModule.getBalance(TEST_IDENTITY_ID);

      // Get full identity
      mockWasmSdk.fetchIdentity.mockResolvedValue({ id: TEST_IDENTITY_ID });
      await identityModule.get(TEST_IDENTITY_ID);

      // Verify all operations used FetchOptions with prove
      const allFetchOptionsCalls = mockWasmSdk.FetchOptions.mock.results;
      expect(allFetchOptionsCalls.length).toBeGreaterThan(0);
      
      allFetchOptionsCalls.forEach(result => {
        expect(result.value.withProve).toHaveBeenCalledWith(true);
      });
    });
  });
});