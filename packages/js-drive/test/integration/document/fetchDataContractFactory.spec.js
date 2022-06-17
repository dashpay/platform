const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('fetchDataContractFactory', () => {
  let fetchDataContract;
  let documentType;
  let contractId;
  let document;
  let dataContractRepository;
  let dataContract;
  let container;

  beforeEach(async () => {
    container = await createTestDIContainer();

    dataContractRepository = container.resolve('dataContractRepository');

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

    const createInitialStateStructure = container.resolve('createInitialStateStructure');
    await createInitialStateStructure();

    await dataContractRepository.store(dataContract);

    fetchDataContract = container.resolve('fetchDataContract');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should fetch DataContract for specified contract ID and document type', async () => {
    const result = await fetchDataContract(contractId, documentType);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDataContract = result.getValue();

    expect(foundDataContract.toObject()).to.deep.equal(dataContract.toObject());
  });

  it('should throw InvalidQueryError if contract ID is not valid', async () => {
    contractId = 'something';

    try {
      await fetchDataContract(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal('invalid data contract ID: Identifier expects Buffer');
    }
  });

  it('should throw InvalidQueryError if contract ID does not exist', async () => {
    contractId = generateRandomIdentifier();

    try {
      await fetchDataContract(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal(`data contract ${contractId} not found`);
    }
  });

  it('should throw InvalidQueryError if type does not exist', async () => {
    documentType = 'Unknown';

    try {
      await fetchDataContract(contractId, documentType);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal('document type Unknown is not defined in the data contract');
    }
  });
});
