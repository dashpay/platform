const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');

const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('fetchDocumentsFactory', () => {
  let fetchDocuments;
  let documentType;
  let contractId;
  let document;
  let dataContractRepository;
  let documentRepository;
  let dataContract;
  let container;
  let blockInfo;

  beforeEach(async () => {
    container = await createTestDIContainer();

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

    blockInfo = {
      height: 1,
      epoch: 0,
      timeMs: 100,
    };

    /**
     * @type {Drive}
     */
    const rsDrive = container.resolve('rsDrive');
    await rsDrive.createInitialStateStructure();

    await dataContractRepository.create(dataContract, blockInfo);

    fetchDocuments = container.resolve('fetchDocuments');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should fetch Documents for specified contract ID and document type', async () => {
    await documentRepository.create(document, blockInfo);

    const result = await fetchDocuments(contractId, documentType);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDocuments = result.getValue();

    expect(foundDocuments).to.be.an('array');
    expect(foundDocuments).to.have.lengthOf(1);

    const [actualDocument] = foundDocuments;

    expect(actualDocument.toObject()).to.deep.equal(document.toObject());
  });

  it('should fetch Documents for specified contract id, document type and name', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['name', '==', document.get('name')]] };

    const result = await fetchDocuments(contractId, documentType, query);

    const foundDocuments = result.getValue();

    expect(foundDocuments).to.be.an('array');
    expect(foundDocuments).to.have.lengthOf(1);

    const [actualDocument] = foundDocuments;

    expect(actualDocument.toObject()).to.deep.equal(document.toObject());
  });

  it('should return empty array for specified contract ID, document type and name not exist', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['name', '==', 'unknown']] };

    const result = await fetchDocuments(contractId, documentType, query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDocuments = result.getValue();

    expect(foundDocuments).to.deep.equal([]);
  });

  it('should fetch documents by an equal date', async () => {
    const indexedDocument = getDocumentsFixture(dataContract)[3];

    await documentRepository.create(indexedDocument, blockInfo);

    const query = {
      where: [
        ['$createdAt', '==', indexedDocument.getCreatedAt().getTime()],
      ],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDocuments = result.getValue();

    expect(foundDocuments[0].toObject()).to.deep.equal(
      indexedDocument.toObject(),
    );
  });

  it('should fetch documents by a date range', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

    await documentRepository.create(indexedDocument, blockInfo);

    const startDate = new Date();
    startDate.setSeconds(startDate.getSeconds() - 10);

    const endDate = new Date();
    endDate.setSeconds(endDate.getSeconds() + 10);

    const query = {
      where: [
        ['$createdAt', '>', startDate.getTime()],
        ['$createdAt', '<=', endDate.getTime()],
      ],
      orderBy: [['$createdAt', 'asc']],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDocuments = result.getValue();

    expect(foundDocuments[0].toObject()).to.deep.equal(
      indexedDocument.toObject(),
    );
  });

  it('should fetch empty array in case date is out of range', async () => {
    const [, , , indexedDocument] = getDocumentsFixture(dataContract);

    await documentRepository.create(indexedDocument, blockInfo);

    const startDate = new Date();
    startDate.setSeconds(startDate.getSeconds() + 10);

    const endDate = new Date();
    endDate.setSeconds(endDate.getSeconds() + 20);

    const query = {
      where: [
        ['$createdAt', '>', startDate.getTime()],
        ['$createdAt', '<=', endDate.getTime()],
      ],
      orderBy: [['$createdAt', 'asc']],
    };

    const result = await fetchDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDocuments = result.getValue();

    expect(foundDocuments).to.have.length(0);
  });

  it('should throw InvalidQueryError if searching by non indexed fields', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['lastName', '==', 'unknown']] };

    try {
      await fetchDocuments(contractId, documentType, blockInfo, query);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
    }
  });

  it('should throw InvalidQueryError if type does not exist', async () => {
    documentType = 'Unknown';

    try {
      await fetchDocuments(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal('document type Unknown is not defined in the data contract');
    }
  });
});
