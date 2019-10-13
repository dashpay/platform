const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Contract = require('../../../lib/contract/Contract');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getContractFixture = require('../../../lib/test/fixtures/getContractFixture');

describe('ContractFacade', () => {
  let dpp;
  let contract;

  beforeEach(() => {
    dpp = new DashPlatformProtocol();

    contract = getContractFixture();
  });

  describe('create', () => {
    it('should create Contract', () => {
      const result = dpp.contract.create(
        contract.getId(),
        contract.getDocuments(),
      );

      expect(result).to.be.an.instanceOf(Contract);

      expect(result.getId()).to.equal(contract.getId());
      expect(result.getDocuments()).to.equal(contract.getDocuments());
    });
  });

  describe('createFromObject', () => {
    it('should create Contract from plain object', () => {
      const result = dpp.contract.createFromObject(contract.toJSON());

      expect(result).to.be.an.instanceOf(Contract);

      expect(result.toJSON()).to.deep.equal(contract.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should create Contract from string', () => {
      const result = dpp.contract.createFromSerialized(contract.serialize());

      expect(result).to.be.an.instanceOf(Contract);

      expect(result.toJSON()).to.deep.equal(contract.toJSON());
    });
  });

  describe('validate', () => {
    it('should validate Contract', () => {
      const result = dpp.contract.validate(contract.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
    });
  });
});
