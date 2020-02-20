const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContract = require('../../../lib/dataContract/DataContract');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

describe('DataContractFacade', () => {
  let dpp;
  let dataContract;

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    dataContract = getDataContractFixture();
  });

  describe('create', () => {
    it('should create DataContract', () => {
      const result = dpp.dataContract.create(
        dataContract.getId(),
        dataContract.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(DataContract);

      expect(result.getId()).to.equal(dataContract.getId());
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
    it('should create DataContractStateTransition from DataContract', () => {
      const stateTransition = new DataContractStateTransition(dataContract);

      const result = dpp.dataContract.createStateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

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
