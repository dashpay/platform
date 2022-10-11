const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const InvalidQueryError = require('../../../lib/document/errors/InvalidQueryError');

const createTestDIContainer = require('../../../lib/test/createTestDIContainer');
const StorageResult = require('../../../lib/storage/StorageResult');

describe('fetchDataContractFactory', () => {
  let fetchDataContract;
  let contractId;
  let dataContractRepository;
  let dataContract;
  let container;

  beforeEach(async () => {
    container = await createTestDIContainer();

    dataContractRepository = container.resolve('dataContractRepository');

    dataContract = getDataContractFixture();

    contractId = dataContract.getId();

    /**
     * @type {Drive}
     */
    const rsDrive = container.resolve('rsDrive');
    await rsDrive.createInitialStateStructure();

    await dataContractRepository.store(dataContract);

    fetchDataContract = container.resolve('fetchDataContract');
  });

  afterEach(async () => {
    if (container) {
      await container.dispose();
    }
  });

  it('should fetch DataContract for specified contract ID and document type', async () => {
    const result = await fetchDataContract(contractId);

    expect(result).to.be.instanceOf(StorageResult);
    expect(result.getOperations().length).to.be.greaterThan(0);

    const foundDataContract = result.getValue();

    expect(foundDataContract.toObject()).to.deep.equal(dataContract.toObject());
  });

  it('should throw InvalidQueryError if contract ID is not valid', async () => {
    contractId = 'something';

    try {
      await fetchDataContract(contractId);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal('invalid data contract ID: Identifier expects Buffer');
    }
  });

  it('should throw InvalidQueryError if contract ID does not exist', async () => {
    contractId = generateRandomIdentifier();

    try {
      await fetchDataContract(contractId);

      expect.fail('should throw InvalidQueryError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidQueryError);
      expect(e.message).to.equal(`data contract ${contractId} not found`);
    }
  });
});
