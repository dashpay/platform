import { ContractModule } from '../../../modules/contracts/ContractModule';
import { SDK } from '../../../SDK';
import * as WasmLoader from '../../../core/WasmLoader';
import { ContractSchema } from '../../../modules/contracts/types';

jest.mock('../../../core/WasmLoader');

describe('ContractModule', () => {
  let sdk: SDK;
  let contractModule: ContractModule;
  
  const mockWasmSdk = {
    WasmSdkBuilder: {
      new_testnet: jest.fn().mockReturnValue({
        build: jest.fn().mockReturnValue({
          free: jest.fn(),
        })
      })
    },
    createDataContract: jest.fn(),
    updateDataContract: jest.fn(),
    fetchDataContract: jest.fn(),
    validateDataContract: jest.fn(),
    FetchOptions: jest.fn().mockImplementation(() => ({
      withProve: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
    DataContractCreateOptions: jest.fn().mockImplementation(() => ({
      withIdentity: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
  };

  const TEST_IDENTITY_ID = global.TEST_IDENTITY_ID;
  const TEST_PRIVATE_KEY = global.TEST_PRIVATE_KEY;
  const TEST_CONTRACT_ID = 'EBioSoFFTDf346ndCMHGmYF8QzgwM8972jG5fL4ndBL7';

  const sampleContractSchema: ContractSchema = {
    nft3d: {
      type: 'object',
      indices: [
        {
          name: 'ownerId',
          properties: [{ ownerId: 'asc' }],
        },
        {
          name: 'createdAt',
          properties: [{ $createdAt: 'desc' }],
        },
      ],
      properties: {
        ownerId: {
          type: 'array',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          position: 0,
        },
        name: {
          type: 'string',
          maxLength: 100,
          position: 1,
        },
        description: {
          type: 'string',
          maxLength: 500,
          position: 2,
        },
        modelUrl: {
          type: 'string',
          format: 'uri',
          maxLength: 255,
          position: 3,
        },
        price: {
          type: 'integer',
          minimum: 0,
          position: 4,
        },
      },
      required: ['ownerId', 'name', 'modelUrl'],
      additionalProperties: false,
    },
    collection: {
      type: 'object',
      indices: [
        {
          name: 'ownerId',
          properties: [{ ownerId: 'asc' }],
        },
      ],
      properties: {
        ownerId: {
          type: 'array',
          byteArray: true,
          minItems: 32,
          maxItems: 32,
          position: 0,
        },
        name: {
          type: 'string',
          maxLength: 100,
          position: 1,
        },
        description: {
          type: 'string',
          maxLength: 1000,
          position: 2,
        },
      },
      required: ['ownerId', 'name'],
      additionalProperties: false,
    },
  };

  beforeEach(async () => {
    jest.clearAllMocks();
    (WasmLoader.loadWasmSdk as jest.Mock).mockResolvedValue(mockWasmSdk);
    
    sdk = new SDK({ network: 'testnet' });
    await sdk.init();
    contractModule = new ContractModule(sdk);
  });

  afterEach(() => {
    sdk.close();
  });

  describe('create', () => {
    it('should create data contract with schema', async () => {
      const mockCreatedContract = {
        id: TEST_CONTRACT_ID,
        ownerId: TEST_IDENTITY_ID,
        schema: sampleContractSchema,
        version: 1,
      };

      mockWasmSdk.createDataContract.mockResolvedValue(mockCreatedContract);

      const contract = await contractModule.create({
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        schema: sampleContractSchema,
      });

      // Verify DataContractCreateOptions was used
      expect(mockWasmSdk.DataContractCreateOptions).toHaveBeenCalled();
      const createOptions = mockWasmSdk.DataContractCreateOptions.mock.results[0].value;
      expect(createOptions.withIdentity).toHaveBeenCalledWith(TEST_IDENTITY_ID);

      expect(mockWasmSdk.createDataContract).toHaveBeenCalledWith(
        expect.any(Object),
        sampleContractSchema,
        TEST_PRIVATE_KEY,
        createOptions
      );

      expect(contract).toEqual(mockCreatedContract);
    });

    it('should validate schema before creation', async () => {
      mockWasmSdk.validateDataContract.mockReturnValue({
        isValid: false,
        errors: ['Invalid property type'],
      });

      const invalidSchema = {
        invalid: {
          type: 'invalid-type', // Invalid type
        },
      };

      await expect(
        contractModule.create({
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
          schema: invalidSchema as any,
        })
      ).rejects.toThrow('Invalid property type');
    });

    it('should clean up options after creation', async () => {
      mockWasmSdk.createDataContract.mockResolvedValue({ id: 'new-contract' });

      await contractModule.create({
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        schema: sampleContractSchema,
      });

      const createOptions = mockWasmSdk.DataContractCreateOptions.mock.results[0].value;
      expect(createOptions.free).toHaveBeenCalled();
    });

    it('should handle insufficient identity balance', async () => {
      mockWasmSdk.createDataContract.mockRejectedValue(
        new Error('Insufficient identity balance')
      );

      await expect(
        contractModule.create({
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
          schema: sampleContractSchema,
        })
      ).rejects.toThrow('Insufficient identity balance');
    });
  });

  describe('get', () => {
    const mockContract = {
      id: TEST_CONTRACT_ID,
      ownerId: TEST_IDENTITY_ID,
      schema: sampleContractSchema,
      version: 1,
    };

    it('should fetch data contract with proved query', async () => {
      mockWasmSdk.fetchDataContract.mockResolvedValue(mockContract);

      const contract = await contractModule.get(TEST_CONTRACT_ID);

      // Verify FetchOptions was created with prove
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      expect(mockWasmSdk.fetchDataContract).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        fetchOptions
      );

      expect(contract).toEqual(mockContract);
    });

    it('should return null for non-existent contract', async () => {
      mockWasmSdk.fetchDataContract.mockResolvedValue(null);

      const contract = await contractModule.get('non-existent-id');

      expect(contract).toBeNull();
    });

    it('should clean up FetchOptions after use', async () => {
      mockWasmSdk.fetchDataContract.mockResolvedValue(mockContract);

      await contractModule.get(TEST_CONTRACT_ID);

      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.free).toHaveBeenCalled();
    });

    it('should validate contract ID format', async () => {
      await expect(
        contractModule.get('invalid-format')
      ).rejects.toThrow();
    });
  });

  describe('update', () => {
    it('should update data contract schema', async () => {
      const updatedSchema = {
        ...sampleContractSchema,
        newDocumentType: {
          type: 'object',
          properties: {
            field: { type: 'string' },
          },
          additionalProperties: false,
        },
      };

      const mockUpdatedContract = {
        id: TEST_CONTRACT_ID,
        ownerId: TEST_IDENTITY_ID,
        schema: updatedSchema,
        version: 2,
      };

      mockWasmSdk.updateDataContract.mockResolvedValue(mockUpdatedContract);

      const updated = await contractModule.update({
        contractId: TEST_CONTRACT_ID,
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        schema: updatedSchema,
      });

      expect(mockWasmSdk.updateDataContract).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        updatedSchema,
        TEST_PRIVATE_KEY
      );

      expect(updated.version).toBe(2);
      expect(updated.schema).toEqual(updatedSchema);
    });

    it('should validate ownership before update', async () => {
      mockWasmSdk.updateDataContract.mockRejectedValue(
        new Error('Not contract owner')
      );

      await expect(
        contractModule.update({
          contractId: TEST_CONTRACT_ID,
          identityId: 'wrong-identity',
          privateKey: TEST_PRIVATE_KEY,
          schema: sampleContractSchema,
        })
      ).rejects.toThrow('Not contract owner');
    });

    it('should validate new schema before update', async () => {
      mockWasmSdk.validateDataContract.mockReturnValue({
        isValid: false,
        errors: ['Cannot remove existing document type'],
      });

      const invalidUpdate = {
        // Missing existing document types
        newType: { type: 'object' },
      };

      await expect(
        contractModule.update({
          contractId: TEST_CONTRACT_ID,
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
          schema: invalidUpdate as any,
        })
      ).rejects.toThrow('Cannot remove existing document type');
    });
  });

  describe('getHistory', () => {
    it('should fetch contract history', async () => {
      const mockHistory = [
        {
          version: 1,
          schema: sampleContractSchema,
          createdAt: '2024-01-01T00:00:00Z',
        },
        {
          version: 2,
          schema: { ...sampleContractSchema, updated: true },
          createdAt: '2024-01-02T00:00:00Z',
        },
      ];

      mockWasmSdk.fetchDataContract.mockImplementation((sdk, id, options) => {
        // Mock different versions based on options
        return mockHistory[0];
      });

      const history = await contractModule.getHistory(TEST_CONTRACT_ID);

      expect(history).toBeDefined();
      // Implementation would depend on how history is stored
    });
  });

  describe('Schema Validation', () => {
    it('should validate document types have required fields', () => {
      const invalidSchema = {
        invalidType: {
          // Missing 'type' field
          properties: {},
        },
      };

      expect(() => {
        contractModule['validateSchemaStructure'](invalidSchema as any);
      }).toThrow();
    });

    it('should validate property positions are unique', () => {
      const invalidSchema = {
        docType: {
          type: 'object',
          properties: {
            field1: { type: 'string', position: 0 },
            field2: { type: 'string', position: 0 }, // Duplicate position
          },
          additionalProperties: false,
        },
      };

      expect(() => {
        contractModule['validateSchemaStructure'](invalidSchema as any);
      }).toThrow();
    });

    it('should validate indices reference existing properties', () => {
      const invalidSchema = {
        docType: {
          type: 'object',
          properties: {
            field1: { type: 'string' },
          },
          indices: [
            {
              name: 'badIndex',
              properties: [{ nonExistentField: 'asc' }], // Non-existent field
            },
          ],
          additionalProperties: false,
        },
      };

      expect(() => {
        contractModule['validateSchemaStructure'](invalidSchema as any);
      }).toThrow();
    });
  });

  describe('Cost Estimation', () => {
    it('should estimate contract creation cost', async () => {
      const estimatedCost = await contractModule.estimateCreationCost(
        sampleContractSchema
      );

      expect(estimatedCost).toBeGreaterThan(0);
      expect(typeof estimatedCost).toBe('number');
    });

    it('should factor in schema complexity', async () => {
      const simpleSchema = {
        simple: {
          type: 'object',
          properties: {
            field: { type: 'string' },
          },
          additionalProperties: false,
        },
      };

      const complexSchema = {
        ...sampleContractSchema,
        extra1: { ...sampleContractSchema.nft3d },
        extra2: { ...sampleContractSchema.collection },
      };

      const simpleCost = await contractModule.estimateCreationCost(simpleSchema);
      const complexCost = await contractModule.estimateCreationCost(complexSchema);

      expect(complexCost).toBeGreaterThan(simpleCost);
    });
  });

  describe('Error Handling', () => {
    it('should handle network errors gracefully', async () => {
      mockWasmSdk.fetchDataContract.mockRejectedValue(
        new Error('Network timeout')
      );

      await expect(
        contractModule.get(TEST_CONTRACT_ID)
      ).rejects.toThrow('Network timeout');
    });

    it('should provide meaningful error for schema violations', async () => {
      const oversizedSchema = {
        docType: {
          type: 'object',
          properties: Object.fromEntries(
            Array.from({ length: 1000 }, (_, i) => [
              `field${i}`,
              { type: 'string' },
            ])
          ),
          additionalProperties: false,
        },
      };

      await expect(
        contractModule.create({
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
          schema: oversizedSchema as any,
        })
      ).rejects.toThrow();
    });
  });

  describe('Integration Tests', () => {
    it('should create and fetch contract', async () => {
      // Create contract
      const mockCreated = {
        id: 'new-contract-id',
        ownerId: TEST_IDENTITY_ID,
        schema: sampleContractSchema,
        version: 1,
      };

      mockWasmSdk.createDataContract.mockResolvedValue(mockCreated);

      const created = await contractModule.create({
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        schema: sampleContractSchema,
      });

      expect(created.id).toBe('new-contract-id');

      // Fetch the created contract
      mockWasmSdk.fetchDataContract.mockResolvedValue(mockCreated);

      const fetched = await contractModule.get('new-contract-id');

      expect(fetched).toEqual(created);
    });

    it('should ensure all operations use proved mode', async () => {
      // Get contract
      mockWasmSdk.fetchDataContract.mockResolvedValue({
        id: TEST_CONTRACT_ID,
        schema: {},
      });
      await contractModule.get(TEST_CONTRACT_ID);

      // Verify all FetchOptions used prove
      const allFetchOptions = mockWasmSdk.FetchOptions.mock.results;
      expect(allFetchOptions.length).toBeGreaterThan(0);
      
      allFetchOptions.forEach(result => {
        expect(result.value.withProve).toHaveBeenCalledWith(true);
      });
    });
  });
});