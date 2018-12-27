const DashApplicationProtocol = require('../../../lib/DashApplicationProtocol');

const DapContract = require('../../../lib/dapContract/DapContract');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

describe('DapContractFacade', () => {
  let dap;
  let dapContract;

  beforeEach(() => {
    dap = new DashApplicationProtocol();

    dapContract = getDapContractFixture();
  });

  describe('create', () => {
    it('should create DAP Contract', () => {
      const result = dap.contract.create(
        dapContract.getName(),
        dapContract.getDapObjectsDefinition(),
      );

      expect(result).to.be.instanceOf(DapContract);

      expect(result.getName()).to.be.equal(dapContract.getName());
      expect(result.getDapObjectsDefinition()).to.be.equal(dapContract.getDapObjectsDefinition());
    });
  });

  describe('createFromObject', () => {
    it('should create DAP Contract from plain object', () => {
      const result = dap.contract.createFromObject(dapContract.toJSON());

      expect(result).to.be.instanceOf(DapContract);

      expect(result.toJSON()).to.be.deep.equal(dapContract.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should create DAP Contract from string', () => {
      const result = dap.contract.createFromSerialized(dapContract.serialize());

      expect(result).to.be.instanceOf(DapContract);

      expect(result.toJSON()).to.be.deep.equal(dapContract.toJSON());
    });
  });

  describe('validate', () => {
    it('should validate DAP Contract', () => {
      const result = dap.contract.validate(dapContract.toJSON());

      expect(result).to.be.instanceOf(ValidationResult);
    });
  });
});
