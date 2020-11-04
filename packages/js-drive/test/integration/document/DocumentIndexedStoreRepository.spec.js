const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('DocumentIndexedStoreRepository', () => {
  let documentIndexedStoreRepository;
  let document;
  let dataContract;
  let container;
  let mongoDb;
  let documentsDbTransaction;
  let documentStoreRepository;
  let createDocumentMongoDbRepository;
  let dataContractRepository;
  let documentDatabaseManager;
  let indicesRepository;

  startMongoDb().then((mongo) => {
    mongoDb = mongo;
  });

  beforeEach(async () => {
    container = await createTestDIContainer(mongoDb);

    documentIndexedStoreRepository = container.resolve('documentRepository');
    documentsDbTransaction = container.resolve('documentsDbTransaction');
    documentStoreRepository = container.resolve('documentStoreRepository');
    createDocumentMongoDbRepository = container.resolve('createDocumentMongoDbRepository');
    dataContractRepository = container.resolve('dataContractRepository');
    documentDatabaseManager = container.resolve('documentDatabaseManager');

    dataContract = getDataContractFixture();

    [document] = getDocumentsFixture(dataContract);

    await dataContractRepository.store(dataContract);
    await documentDatabaseManager.create(dataContract);

    indicesRepository = await createDocumentMongoDbRepository(
      document.getDataContractId(),
      document.getType(),
    );
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  describe('#store', () => {
    it('should store document with transaction', async () => {
      await documentsDbTransaction.start();

      await documentIndexedStoreRepository.store(document, documentsDbTransaction);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const nonTransactionalStoredDocument = await documentStoreRepository.fetch(document.getId());
      const nonTransactionalDocumentIds = await indicesRepository.find(query);

      expect(nonTransactionalStoredDocument).to.be.null();
      expect(nonTransactionalDocumentIds).to.have.lengthOf(0);

      const storedDocument = await documentStoreRepository.fetch(
        document.getId(),
        documentsDbTransaction.getStoreTransaction(),
      );
      const documentIds = await indicesRepository.find(
        query,
        documentsDbTransaction.getMongoDbTransaction(),
      );

      expect(storedDocument.toObject()).to.deep.equal(document.toObject());
      expect(documentIds).to.have.lengthOf(1);

      const [documentId] = documentIds;

      expect(documentId).to.deep.equal(document.getId());

      await documentsDbTransaction.commit();

      const committedStoredDocument = await documentStoreRepository.fetch(document.getId());
      const committedDocumentIds = await indicesRepository.find(query);

      expect(committedStoredDocument.toObject()).to.deep.equal(document.toObject());
      expect(documentIds).to.have.lengthOf(1);

      const [committedDocumentId] = committedDocumentIds;

      expect(committedDocumentId).to.deep.equal(document.getId());
    });

    it('should store document without transaction', async () => {
      await documentIndexedStoreRepository.store(document);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const storedDocument = await documentStoreRepository.fetch(document.getId());
      const documentIds = await indicesRepository.find(query);

      expect(storedDocument).to.be.not.null();
      expect(storedDocument.toObject()).to.deep.equal(document.toObject());
      expect(documentIds).to.have.lengthOf(1);

      const [documentId] = documentIds;

      expect(documentId).to.deep.equal(document.getId());
    });
  });

  describe('#fetch', () => {
    it('should fetch document with transaction', async () => {
      await documentsDbTransaction.start();

      await documentStoreRepository.store(document, documentsDbTransaction.getStoreTransaction());

      const nonTransactionalDocument = await documentIndexedStoreRepository.fetch(document.getId());

      expect(nonTransactionalDocument).to.be.null();

      const fetchedDocument = await documentIndexedStoreRepository.fetch(
        document.getId(),
        documentsDbTransaction,
      );

      expect(fetchedDocument).to.be.not.null();
      expect(fetchedDocument.toObject()).to.deep.equal(document.toObject());

      await documentsDbTransaction.commit();

      const committedDocument = await documentIndexedStoreRepository.fetch(document.getId());

      expect(committedDocument).to.be.not.null();
      expect(committedDocument.toObject()).to.deep.equal(document.toObject());
    });

    it('should fetch document without transaction', async () => {
      await documentStoreRepository.store(document);

      const fetchedDocument = await documentIndexedStoreRepository.fetch(document.getId());

      expect(fetchedDocument).to.be.not.null();
      expect(fetchedDocument.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe('#find', () => {
    it('should find document with transaction', async () => {
      await documentsDbTransaction.start();

      await documentStoreRepository.store(document, documentsDbTransaction.getStoreTransaction());
      await indicesRepository.store(document, documentsDbTransaction.getMongoDbTransaction());

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const nonTransactionalDocuments = await documentIndexedStoreRepository.find(
        dataContract.getId(),
        document.getType(),
        query,
      );

      expect(nonTransactionalDocuments).to.have.lengthOf(0);

      const documents = await documentIndexedStoreRepository.find(
        dataContract.getId(),
        document.getType(),
        query,
        documentsDbTransaction,
      );

      expect(documents).to.have.lengthOf(1);

      const [foundDocument] = documents;

      expect(foundDocument.toObject()).to.deep.equal(document.toObject());

      await documentsDbTransaction.commit();

      const committedDocuments = await documentIndexedStoreRepository.find(
        dataContract.getId(),
        document.getType(),
        query,
      );

      expect(committedDocuments).to.have.lengthOf(1);

      const [committedDocument] = documents;

      expect(committedDocument.toObject()).to.deep.equal(document.toObject());
    });

    it('should find document without transaction', async () => {
      await documentStoreRepository.store(document);
      await indicesRepository.store(document);

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const documents = await documentIndexedStoreRepository.find(
        dataContract.getId(),
        document.getType(),
        query,
      );

      expect(documents).to.have.lengthOf(1);

      const [foundDocument] = documents;

      expect(foundDocument.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe('#delete', () => {
    beforeEach(() => {
      const documentsStore = container.resolve('documentsStore');

      documentsStore
        .db
        .batch()
        // MerkDB doesn't delete the last key for some reason
        // So we need to add an extra one to test delete functionality
        // on empty database
        .put(Buffer.alloc(1), Buffer.alloc(1))
        .commitSync();
    });

    it('should delete document with transaction', async () => {
      await documentStoreRepository.store(document);
      await indicesRepository.store(document);

      await documentsDbTransaction.start();

      await documentIndexedStoreRepository.delete(
        dataContract.getId(),
        document.getType(),
        document.getId(),
        documentsDbTransaction,
      );

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const nonTransactionalStoredDocument = await documentStoreRepository.fetch(document.getId());
      const nonTransactionalDocumentIds = await indicesRepository.find(query);

      expect(nonTransactionalStoredDocument).to.be.not.null();
      expect(nonTransactionalDocumentIds).to.have.lengthOf(1);

      const transactionalStoredDocument = await documentStoreRepository.fetch(
        document.getId(),
        documentsDbTransaction.getStoreTransaction(),
      );
      const transactionalDocumentIds = await indicesRepository.find(
        query,
        documentsDbTransaction.getMongoDbTransaction(),
      );

      expect(transactionalStoredDocument).to.be.null();
      expect(transactionalDocumentIds).to.have.lengthOf(0);

      await documentsDbTransaction.commit();

      const committedStoredDocument = await documentStoreRepository.fetch(document.getId());
      const committedDocumentIds = await indicesRepository.find(query);

      expect(committedStoredDocument).to.be.null();
      expect(committedDocumentIds).to.have.lengthOf(0);
    });

    it('should delete document without transaction', async () => {
      await documentStoreRepository.store(document);
      await indicesRepository.store(document);

      await documentIndexedStoreRepository.delete(
        dataContract.getId(),
        document.getType(),
        document.getId(),
      );

      const query = {
        where: [['$id', '==', document.getId()]],
      };

      const storedDocument = await documentStoreRepository.fetch(document.getId());
      const documentIds = await indicesRepository.find(query);

      expect(storedDocument).to.be.null();
      expect(documentIds).to.have.lengthOf(0);
    });
  });
});
