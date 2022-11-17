const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const StorageResult = require('../../../lib/storage/StorageResult');
const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('proveDocumentsFactory', () => {
  let proveDocuments;
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

    blockInfo = new BlockInfo(1, 0, Date.now());

    /**
     * @type {Drive}
     */
    const rsDrive = container.resolve('rsDrive');
    await rsDrive.createInitialStateStructure();

    await dataContractRepository.create(dataContract, blockInfo);

    proveDocuments = container.resolve('proveDocuments');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should return proof for specified contract ID and document type', async () => {
    await documentRepository.create(document, blockInfo);

    const result = await proveDocuments(contractId, documentType);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
  });

  it('should return proof for specified contract id, document type and name', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['name', '==', document.get('name')]] };

    const result = await proveDocuments(contractId, documentType, query);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
  });

  it('should return proof for specified contract ID, document type and name not exist', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['name', '==', 'unknown']] };

    const result = await proveDocuments(contractId, documentType, query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
  });

  it('should return proof by an equal date', async () => {
    const indexedDocument = getDocumentsFixture(dataContract)[3];

    await documentRepository.create(indexedDocument, blockInfo);

    const query = {
      where: [
        ['$createdAt', '==', indexedDocument.getCreatedAt().getTime()],
      ],
    };

    const result = await proveDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
  });

  it('should return proof by a date range', async () => {
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

    const result = await proveDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
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

    const result = await proveDocuments(contractId, 'indexedDocument', query);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const proof = result.getValue();

    expect(proof).to.be.an.instanceOf(Buffer);
    expect(proof.length).to.be.greaterThan(0);
  });

  it('should throw InvalidQueryError if searching by non indexed fields', async () => {
    await documentRepository.create(document, blockInfo);

    const query = { where: [['lastName', '==', 'unknown']] };

    try {
      await proveDocuments(contractId, documentType, query);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
    }
  });
});
