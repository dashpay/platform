const rimraf = require('rimraf');
const merk = require('merk');

const LRUCache = require('lru-cache');

const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const convertWhereToMongoDbQuery = require('../../../lib/document/mongoDbRepository/convertWhereToMongoDbQuery');
const validateQueryFactory = require('../../../lib/document/query/validateQueryFactory');
const findConflictingConditions = require('../../../lib/document/query/findConflictingConditions');
const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

const createDocumentMongoDbRepositoryFactory = require('../../../lib/document/mongoDbRepository/createDocumentMongoDbRepositoryFactory');
const fetchDocumentsFactory = require('../../../lib/document/fetchDocumentsFactory');
const DataContractStoreRepository = require('../../../lib/dataContract/DataContractStoreRepository');
const getDocumentDatabaseFactory = require('../../../lib/document/mongoDbRepository/getDocumentDatabaseFactory');

const findNotIndexedFields = require('../../../lib/document/query/findNotIndexedFields');
const findNotIndexedOrderByFields = require('../../../lib/document/query/findNotIndexedOrderByFields');
const getIndexedFieldsFromDocumentSchema = require('../../../lib/document/query/getIndexedFieldsFromDocumentSchema');

const MerkDbStore = require('../../../lib/merkDb/MerkDbStore');

describe('fetchDocumentsFactory', () => {
  let createDocumentMongoDbRepository;
  let fetchDocuments;
  let mongoClient;
  let documentType;
  let contractId;
  let document;
  let dataContractRepository;
  let dataContract;
  let dataContractsMerkDbPath;

  startMongoDb().then((mongoDb) => {
    mongoClient = mongoDb.getClient();
  });

  beforeEach(async () => {
    dataContractsMerkDbPath = './db/merkdb-test';

    const dataContractsMerkDb = merk(`${dataContractsMerkDbPath}/${Math.random()}`);
    const dataContractsStore = new MerkDbStore(dataContractsMerkDb);

    const validateQuery = validateQueryFactory(
      findConflictingConditions,
      getIndexedFieldsFromDocumentSchema,
      findNotIndexedFields,
      findNotIndexedOrderByFields,
    );

    const documentsMongoDBPrefix = 'test';
    const connectToDocumentMongoDB = async () => mongoClient;

    const getDocumentDatabase = getDocumentDatabaseFactory(
      connectToDocumentMongoDB,
      documentsMongoDBPrefix,
    );

    dataContractRepository = new DataContractStoreRepository(
      dataContractsStore,
      new DashPlatformProtocol(),
    );

    dataContract = getDataContractFixture();

    contractId = dataContract.getId();

    [document] = getDocumentsFixture(dataContract);

    documentType = document.getType();

    dataContract.documents[documentType].indices = [
      {
        properties: [
          { name: 'asc' },
        ],
      },
    ];

    await dataContractRepository.store(dataContract);

    const blockExecutionDBTransactionsMock = {
      getTransaction: () => ({
        isStarted: () => false,
      }),
    };

    createDocumentMongoDbRepository = createDocumentMongoDbRepositoryFactory(
      convertWhereToMongoDbQuery,
      validateQuery,
      getDocumentDatabase,
      dataContractRepository,
      blockExecutionDBTransactionsMock,
    );

    const dataContractCache = new LRUCache(500);

    fetchDocuments = fetchDocumentsFactory(
      createDocumentMongoDbRepository,
      dataContractRepository,
      dataContractCache,
    );
  });

  afterEach(async () => {
    rimraf.sync(dataContractsMerkDbPath);
  });

  it('should fetch Documents for specified contract ID and document type', async () => {
    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);
    await documentRepository.store(document);

    const result = await fetchDocuments(contractId, documentType);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    expect(actualDocument.toObject()).to.deep.equal(document.toObject());
  });

  it('should fetch Documents for specified contract id, document type and name', async () => {
    let result = await fetchDocuments(contractId, documentType);

    expect(result).to.deep.equal([]);

    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);
    await documentRepository.store(document);

    const query = { where: [['name', '==', document.get('name')]] };
    result = await fetchDocuments(contractId, documentType, query);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    expect(actualDocument.toObject()).to.deep.equal(document.toObject());
  });

  it('should return empty array for specified contract ID, document type and name not exist', async () => {
    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);
    await documentRepository.store(document);

    const query = { where: [['name', '==', 'unknown']] };

    const result = await fetchDocuments(contractId, documentType, query);

    expect(result).to.deep.equal([]);
  });

  it('should fetch documents by an equal date', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

    const documentRepository = await createDocumentMongoDbRepository(contractId, 'indexedDocument');
    await documentRepository.store(indexedDocument);

    const query = {
      where: [
        ['$createdAt', '==', indexedDocument.getCreatedAt().getTime()],
      ],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result[0].toObject()).to.deep.equal(
      indexedDocument.toObject(),
    );
  });

  it('should fetch documents by a date range', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

    const documentRepository = await createDocumentMongoDbRepository(contractId, 'indexedDocument');
    await documentRepository.store(indexedDocument);

    const startDate = new Date();
    startDate.setSeconds(startDate.getSeconds() - 10);

    const endDate = new Date();
    endDate.setSeconds(endDate.getSeconds() + 10);

    const query = {
      where: [
        ['$createdAt', '>', startDate.getTime()],
        ['$createdAt', '<=', endDate.getTime()],
      ],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result[0].toObject()).to.deep.equal(
      indexedDocument.toObject(),
    );
  });

  it('should fetch empty array in case date is out of range', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

    const documentRepository = await createDocumentMongoDbRepository(contractId, 'indexedDocument');
    await documentRepository.store(indexedDocument);

    const startDate = new Date();
    startDate.setSeconds(startDate.getSeconds() + 10);

    const endDate = new Date();
    endDate.setSeconds(endDate.getSeconds() + 20);

    const query = {
      where: [
        ['$createdAt', '>', startDate.getTime()],
        ['$createdAt', '<=', endDate.getTime()],
      ],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result).to.have.length(0);
  });

  it('should throw InvalidQueryError if contract ID is not valid', async () => {
    contractId = 'something';

    try {
      await fetchDocuments(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.getErrors()).to.be.an('array');
      expect(e.getErrors()).to.have.lengthOf(1);

      const [error] = e.getErrors();

      expect(error.getContractId()).to.be.deep.equal(contractId);
    }
  });

  it('should throw InvalidQueryError if contract ID does not exist', async () => {
    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);
    await documentRepository.store(document);

    contractId = generateRandomIdentifier();

    try {
      await fetchDocuments(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.getErrors()).to.be.an('array');
      expect(e.getErrors()).to.have.lengthOf(1);

      const [error] = e.getErrors();

      expect(error.getContractId()).to.be.deep.equal(contractId);
    }
  });

  it('should throw InvalidQueryError if type does not exist', async () => {
    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);

    await documentRepository.store(document);

    documentType = 'Unknown';

    try {
      await fetchDocuments(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.getErrors()).to.be.an('array');
      expect(e.getErrors()).to.have.lengthOf(1);

      const [error] = e.getErrors();

      expect(error.getDocumentType()).to.be.equal(documentType);
    }
  });

  it('should throw InvalidQueryError if searching by non indexed fields', async () => {
    const documentRepository = await createDocumentMongoDbRepository(contractId, documentType);
    await documentRepository.store(document);

    const query = { where: [['lastName', '==', 'unknown']] };

    try {
      await fetchDocuments(contractId, documentType, query);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.getErrors()).to.be.an('array');
      expect(e.getErrors()).to.have.lengthOf(1);

      const [error] = e.getErrors();

      expect(error.getNotIndexedField()).to.be.equal('lastName');
    }
  });
});
