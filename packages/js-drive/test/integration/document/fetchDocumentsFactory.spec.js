const { mocha: { startMongoDb } } = require('@dashevo/dp-services-ctl');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');

describe('fetchDocumentsFactory', () => {
  let fetchDocuments;
  let documentType;
  let contractId;
  let document;
  let dataContractRepository;
  let documentRepository;
  let dataContract;
  let container;
  let mongoDb;

  startMongoDb().then((mongo) => {
    mongoDb = mongo;
  });

  beforeEach(async () => {
    container = createTestDIContainer(mongoDb);

    dataContractRepository = container.resolve('dataContractRepository');
    documentRepository = container.resolve('documentRepository');

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

    const blockExecutionStoreTransactions = container.resolve('blockExecutionStoreTransactions');
    const dataContractsTransaction = blockExecutionStoreTransactions.getTransaction('dataContracts');

    await dataContractsTransaction.start();

    await dataContractRepository.store(dataContract);

    fetchDocuments = container.resolve('fetchDocuments');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should fetch Documents for specified contract ID and document type', async () => {
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

    await documentRepository.store(document);

    const query = { where: [['name', '==', document.get('name')]] };
    result = await fetchDocuments(contractId, documentType, query);

    expect(result).to.be.an('array');
    expect(result).to.have.lengthOf(1);

    const [actualDocument] = result;

    expect(actualDocument.toObject()).to.deep.equal(document.toObject());
  });

  it('should return empty array for specified contract ID, document type and name not exist', async () => {
    await documentRepository.store(document);

    const query = { where: [['name', '==', 'unknown']] };

    const result = await fetchDocuments(contractId, documentType, query);

    expect(result).to.deep.equal([]);
  });

  it('should fetch documents by an equal date', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

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
