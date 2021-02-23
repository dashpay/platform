const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');

describe('BlockExecutionContext', () => {
  let blockExecutionContext;
  let dataContract;

  beforeEach(() => {
    blockExecutionContext = new BlockExecutionContext();
    dataContract = getDataContractFixture();
  });

  describe('#addDataContract', () => {
    it('should add a Data Contract', async () => {
      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(0);

      blockExecutionContext.addDataContract(dataContract);
      const contracts = blockExecutionContext.getDataContracts();

      expect(contracts).to.have.lengthOf(1);
      expect(contracts[0]).to.deep.equal(dataContract);
    });
  });

  describe('#hasDataContract', () => {
    it('should respond with false if data contract with specified ID is not present', async () => {
      const result = blockExecutionContext.hasDataContract(dataContract.getId());

      expect(result).to.be.false();
    });

    it('should respond with true if data contract with specified ID is present', async () => {
      blockExecutionContext.addDataContract(dataContract);

      const result = blockExecutionContext.hasDataContract(dataContract.getId());

      expect(result).to.be.true();
    });
  });

  describe('#getDataContracts', () => {
    it('should get data contracts', async () => {
      blockExecutionContext.addDataContract(dataContract);
      blockExecutionContext.addDataContract(dataContract);

      const contracts = blockExecutionContext.getDataContracts();

      expect(contracts).to.have.lengthOf(2);
      expect(contracts[0]).to.deep.equal(dataContract);
      expect(contracts[1]).to.deep.equal(dataContract);
    });
  });

  describe('#getCumulativeFees', () => {
    it('should get cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeFees();

      expect(result).to.equal(0);

      blockExecutionContext.cumulativeFees = 10;

      result = blockExecutionContext.getCumulativeFees();

      expect(result).to.equal(10);
    });
  });

  describe('#incrementCumulativeFees', () => {
    it('should increment cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeFees();

      expect(result).to.equal(0);

      blockExecutionContext.incrementCumulativeFees(15);

      result = blockExecutionContext.getCumulativeFees();

      expect(result).to.equal(15);
    });
  });

  describe('#reset', () => {
    it('should reset state', () => {
      blockExecutionContext.addDataContract(dataContract);

      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(1);

      blockExecutionContext.reset();

      expect(blockExecutionContext.getDataContracts()).to.have.lengthOf(0);

      expect(blockExecutionContext.getHeader()).to.be.null();
    });
  });

  describe('#setHeader', () => {
    it('should set header', async () => {
      const header = {
        height: 10,
        time: {
          seconds: Math.ceil(new Date().getTime() / 1000),
        },
      };

      const result = blockExecutionContext.setHeader(header);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.header).to.deep.equal(header);
    });
  });

  describe('#getHeader', () => {
    it('should get header', async () => {
      const header = {
        height: 10,
        time: {
          seconds: Math.ceil(new Date().getTime() / 1000),
        },
      };

      blockExecutionContext.header = header;

      expect(blockExecutionContext.getHeader()).to.deep.equal(header);
    });
  });
});
