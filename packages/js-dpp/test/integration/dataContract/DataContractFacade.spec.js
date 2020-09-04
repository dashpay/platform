const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContract = require('../../../lib/dataContract/DataContract');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const DataContractFactory = require('../../../lib/dataContract/DataContractFactory');

describe('DataContractFacade', () => {
  let dpp;
  let dataContract;
  let dataContractFactory;

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    dataContract = getDataContractFixture();

    dataContractFactory = new DataContractFactory(undefined);
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContract.getOwnerId(),
        dataContract.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getOwnerId()).to.equal(dataContract.getOwnerId());
      expect(result.getDocuments()).to.equal(dataContract.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create DataContract from plain object', async () => {
      const result = await dpp.dataContract.createFromObject(dataContract.toJSON());

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toJSON()).to.deep.equal(dataContract.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should create DataContract from string', async () => {
      const result = await dpp.dataContract.createFromSerialized(dataContract.serialize());

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.toJSON()).to.deep.equal(dataContract.toJSON());
    });
  });

  describe('createStateTransition', () => {
    it('should create DataContractCreateTransition from DataContract', () => {
      const stateTransition = dataContractFactory.createStateTransition(dataContract);

      const result = dpp.dataContract.createStateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toJSON()).to.deep.equal(stateTransition.toJSON());
    });
  });

  describe('validate', () => {
    it('should validate DataContract', async () => {
      const result = await dpp.dataContract.validate(dataContract.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
