import { DocumentModule } from '../../../modules/documents/DocumentModule';
import { SDK } from '../../../SDK';
import * as WasmLoader from '../../../core/WasmLoader';
import { DocumentQuery, DocumentCreateOptions } from '../../../modules/documents/types';

jest.mock('../../../core/WasmLoader');

describe('DocumentModule', () => {
  let sdk: SDK;
  let documentModule: DocumentModule;
  
  const mockWasmSdk = {
    WasmSdkBuilder: {
      new_testnet: jest.fn().mockReturnValue({
        build: jest.fn().mockReturnValue({
          free: jest.fn(),
        })
      })
    },
    fetchDocuments: jest.fn(),
    createDocument: jest.fn(),
    updateDocument: jest.fn(),
    deleteDocument: jest.fn(),
    FetchOptions: jest.fn().mockImplementation(() => ({
      withProve: jest.fn().mockReturnThis(),
      withLimit: jest.fn().mockReturnThis(),
      withStartAfter: jest.fn().mockReturnThis(),
      withOrderBy: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
    DocumentOptions: jest.fn().mockImplementation(() => ({
      withOwner: jest.fn().mockReturnThis(),
      withTimestamp: jest.fn().mockReturnThis(),
      free: jest.fn(),
    })),
  };

  const TEST_CONTRACT_ID = 'EBioSoFFTDf346ndCMHGmYF8QzgwM8972jG5fL4ndBL7';
  const TEST_IDENTITY_ID = global.TEST_IDENTITY_ID;
  const TEST_PRIVATE_KEY = global.TEST_PRIVATE_KEY;

  beforeEach(async () => {
    jest.clearAllMocks();
    (WasmLoader.loadWasmSdk as jest.Mock).mockResolvedValue(mockWasmSdk);
    
    sdk = new SDK({ network: 'testnet' });
    await sdk.init();
    documentModule = new DocumentModule(sdk);
  });

  afterEach(() => {
    sdk.close();
  });

  describe('query', () => {
    const mockDocuments = [
      {
        id: 'doc1',
        type: 'nft3d',
        ownerId: TEST_IDENTITY_ID,
        data: {
          name: 'Test NFT 1',
          description: 'A test NFT',
          modelUrl: 'https://example.com/model1.glb',
        },
        revision: 1,
      },
      {
        id: 'doc2',
        type: 'nft3d',
        ownerId: TEST_IDENTITY_ID,
        data: {
          name: 'Test NFT 2',
          description: 'Another test NFT',
          modelUrl: 'https://example.com/model2.glb',
        },
        revision: 1,
      },
    ];

    it('should query documents with proved mode', async () => {
      mockWasmSdk.fetchDocuments.mockResolvedValue(mockDocuments);

      const query: DocumentQuery = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
      };

      const documents = await documentModule.query(query);

      // Verify FetchOptions was created with prove
      expect(mockWasmSdk.FetchOptions).toHaveBeenCalled();
      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withProve).toHaveBeenCalledWith(true);

      // Verify the WASM call
      expect(mockWasmSdk.fetchDocuments).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        'nft3d',
        fetchOptions
      );

      expect(documents).toEqual(mockDocuments);
    });

    it('should apply query options', async () => {
      mockWasmSdk.fetchDocuments.mockResolvedValue([]);

      const query: DocumentQuery = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        limit: 10,
        startAfter: 'lastDocId',
        orderBy: [['createdAt', 'desc']],
      };

      await documentModule.query(query);

      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.withLimit).toHaveBeenCalledWith(10);
      expect(fetchOptions.withStartAfter).toHaveBeenCalledWith('lastDocId');
      expect(fetchOptions.withOrderBy).toHaveBeenCalledWith([['createdAt', 'desc']]);
    });

    it('should query with where conditions', async () => {
      mockWasmSdk.fetchDocuments.mockResolvedValue(mockDocuments);

      const query: DocumentQuery = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        where: [
          ['ownerId', '==', TEST_IDENTITY_ID],
          ['data.name', 'startsWith', 'Test'],
        ],
      };

      const documents = await documentModule.query(query);

      expect(mockWasmSdk.fetchDocuments).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        'nft3d',
        expect.any(Object)
      );

      expect(documents).toHaveLength(2);
    });

    it('should handle empty results', async () => {
      mockWasmSdk.fetchDocuments.mockResolvedValue([]);

      const documents = await documentModule.query({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
      });

      expect(documents).toEqual([]);
    });

    it('should clean up FetchOptions after use', async () => {
      mockWasmSdk.fetchDocuments.mockResolvedValue([]);

      await documentModule.query({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
      });

      const fetchOptions = mockWasmSdk.FetchOptions.mock.results[0].value;
      expect(fetchOptions.free).toHaveBeenCalled();
    });
  });

  describe('create', () => {
    const newDocument = {
      name: 'New NFT',
      description: 'A newly created NFT',
      modelUrl: 'https://example.com/new-model.glb',
      price: 100000000, // 1 DASH
    };

    it('should create document with identity', async () => {
      const mockCreatedDoc = {
        id: 'new-doc-id',
        type: 'nft3d',
        ownerId: TEST_IDENTITY_ID,
        data: newDocument,
        revision: 1,
      };

      mockWasmSdk.createDocument.mockResolvedValue(mockCreatedDoc);

      const options: DocumentCreateOptions = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        data: newDocument,
      };

      const created = await documentModule.create(options);

      // Verify DocumentOptions was created
      expect(mockWasmSdk.DocumentOptions).toHaveBeenCalled();
      const docOptions = mockWasmSdk.DocumentOptions.mock.results[0].value;
      expect(docOptions.withOwner).toHaveBeenCalledWith(TEST_IDENTITY_ID);

      expect(mockWasmSdk.createDocument).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        'nft3d',
        newDocument,
        TEST_PRIVATE_KEY,
        docOptions
      );

      expect(created).toEqual(mockCreatedDoc);
    });

    it('should add timestamp if requested', async () => {
      mockWasmSdk.createDocument.mockResolvedValue({ id: 'new-doc' });

      const options: DocumentCreateOptions = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        data: newDocument,
        addTimestamp: true,
      };

      await documentModule.create(options);

      const docOptions = mockWasmSdk.DocumentOptions.mock.results[0].value;
      expect(docOptions.withTimestamp).toHaveBeenCalledWith(true);
    });

    it('should validate required fields', async () => {
      const invalidOptions = {
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        // Missing identityId and privateKey
        data: newDocument,
      } as DocumentCreateOptions;

      await expect(
        documentModule.create(invalidOptions)
      ).rejects.toThrow('Identity ID required');
    });

    it('should clean up DocumentOptions after use', async () => {
      mockWasmSdk.createDocument.mockResolvedValue({ id: 'new-doc' });

      await documentModule.create({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        data: newDocument,
      });

      const docOptions = mockWasmSdk.DocumentOptions.mock.results[0].value;
      expect(docOptions.free).toHaveBeenCalled();
    });
  });

  describe('update', () => {
    const documentId = 'existing-doc-id';
    const updateData = {
      price: 200000000, // 2 DASH
      description: 'Updated description',
    };

    it('should update document', async () => {
      const mockUpdatedDoc = {
        id: documentId,
        type: 'nft3d',
        ownerId: TEST_IDENTITY_ID,
        data: {
          name: 'Test NFT',
          ...updateData,
        },
        revision: 2,
      };

      mockWasmSdk.updateDocument.mockResolvedValue(mockUpdatedDoc);

      const updated = await documentModule.update({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        documentId,
        data: updateData,
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
      });

      expect(mockWasmSdk.updateDocument).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        'nft3d',
        documentId,
        updateData,
        TEST_PRIVATE_KEY
      );

      expect(updated).toEqual(mockUpdatedDoc);
      expect(updated.revision).toBe(2);
    });

    it('should handle document not found', async () => {
      mockWasmSdk.updateDocument.mockRejectedValue(
        new Error('Document not found')
      );

      await expect(
        documentModule.update({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
          documentId: 'non-existent',
          data: updateData,
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Document not found');
    });

    it('should validate ownership before update', async () => {
      mockWasmSdk.updateDocument.mockRejectedValue(
        new Error('Not document owner')
      );

      await expect(
        documentModule.update({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
          documentId,
          data: updateData,
          identityId: 'different-identity',
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Not document owner');
    });
  });

  describe('delete', () => {
    const documentId = 'doc-to-delete';

    it('should delete document', async () => {
      mockWasmSdk.deleteDocument.mockResolvedValue({
        success: true,
        transitionHash: 'delete-transition-hash',
      });

      const result = await documentModule.delete({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        documentId,
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
      });

      expect(mockWasmSdk.deleteDocument).toHaveBeenCalledWith(
        expect.any(Object),
        TEST_CONTRACT_ID,
        'nft3d',
        documentId,
        TEST_PRIVATE_KEY
      );

      expect(result.success).toBe(true);
    });

    it('should handle deletion of non-existent document', async () => {
      mockWasmSdk.deleteDocument.mockRejectedValue(
        new Error('Document not found')
      );

      await expect(
        documentModule.delete({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
          documentId: 'non-existent',
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Document not found');
    });

    it('should require ownership for deletion', async () => {
      mockWasmSdk.deleteDocument.mockRejectedValue(
        new Error('Not document owner')
      );

      await expect(
        documentModule.delete({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
          documentId,
          identityId: 'wrong-identity',
          privateKey: TEST_PRIVATE_KEY,
        })
      ).rejects.toThrow('Not document owner');
    });
  });

  describe('Batch Operations', () => {
    it('should support batch queries', async () => {
      const mockBatchResults = {
        nft3d: [
          { id: 'nft1', type: 'nft3d' },
          { id: 'nft2', type: 'nft3d' },
        ],
        collection: [
          { id: 'col1', type: 'collection' },
        ],
      };

      // Mock multiple calls for different document types
      mockWasmSdk.fetchDocuments
        .mockResolvedValueOnce(mockBatchResults.nft3d)
        .mockResolvedValueOnce(mockBatchResults.collection);

      const queries = [
        { contractId: TEST_CONTRACT_ID, type: 'nft3d' },
        { contractId: TEST_CONTRACT_ID, type: 'collection' },
      ];

      const results = await Promise.all(
        queries.map(q => documentModule.query(q))
      );

      expect(results[0]).toEqual(mockBatchResults.nft3d);
      expect(results[1]).toEqual(mockBatchResults.collection);

      // Verify all queries used proved mode
      const allFetchOptions = mockWasmSdk.FetchOptions.mock.results;
      allFetchOptions.forEach(result => {
        expect(result.value.withProve).toHaveBeenCalledWith(true);
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle network errors gracefully', async () => {
      mockWasmSdk.fetchDocuments.mockRejectedValue(
        new Error('Network timeout')
      );

      await expect(
        documentModule.query({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
        })
      ).rejects.toThrow('Network timeout');
    });

    it('should validate contract ID format', async () => {
      await expect(
        documentModule.query({
          contractId: 'invalid-format',
          type: 'nft3d',
        })
      ).rejects.toThrow();
    });

    it('should handle invalid document data', async () => {
      await expect(
        documentModule.create({
          contractId: TEST_CONTRACT_ID,
          type: 'nft3d',
          identityId: TEST_IDENTITY_ID,
          privateKey: TEST_PRIVATE_KEY,
          data: null as any, // Invalid data
        })
      ).rejects.toThrow();
    });
  });

  describe('Integration Tests', () => {
    it('should perform full CRUD cycle', async () => {
      // Create
      const createData = {
        name: 'CRUD Test NFT',
        description: 'Testing CRUD operations',
        modelUrl: 'https://example.com/crud-test.glb',
      };

      const mockCreated = {
        id: 'crud-test-id',
        type: 'nft3d',
        ownerId: TEST_IDENTITY_ID,
        data: createData,
        revision: 1,
      };

      mockWasmSdk.createDocument.mockResolvedValue(mockCreated);

      const created = await documentModule.create({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
        data: createData,
      });

      expect(created.id).toBe('crud-test-id');

      // Query
      mockWasmSdk.fetchDocuments.mockResolvedValue([mockCreated]);
      
      const queried = await documentModule.query({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        where: [['id', '==', 'crud-test-id']],
      });

      expect(queried).toHaveLength(1);
      expect(queried[0].id).toBe('crud-test-id');

      // Update
      const updateData = { description: 'Updated description' };
      const mockUpdated = { ...mockCreated, data: { ...createData, ...updateData }, revision: 2 };
      
      mockWasmSdk.updateDocument.mockResolvedValue(mockUpdated);

      const updated = await documentModule.update({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        documentId: 'crud-test-id',
        data: updateData,
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
      });

      expect(updated.revision).toBe(2);
      expect(updated.data.description).toBe('Updated description');

      // Delete
      mockWasmSdk.deleteDocument.mockResolvedValue({ success: true });

      const deleted = await documentModule.delete({
        contractId: TEST_CONTRACT_ID,
        type: 'nft3d',
        documentId: 'crud-test-id',
        identityId: TEST_IDENTITY_ID,
        privateKey: TEST_PRIVATE_KEY,
      });

      expect(deleted.success).toBe(true);
    });
  });
});