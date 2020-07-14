const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const BlockExecutionState = require('../../../../lib/blockchainState/blockExecution/BlockExecutionState');

describe('BlockExecutionState', () => {
  let blockExecutionState;
  let dataContract;

  beforeEach(() => {
    blockExecutionState = new BlockExecutionState();
    dataContract = getDataContractFixture();
  });

  describe('#addDataContract', () => {
    it('should add a Data Contract', async () => {
      expect(blockExecutionState.getDataContracts()).to.have.lengthOf(0);

      blockExecutionState.addDataContract(dataContract);
      const contracts = blockExecutionState.getDataContracts();

      expect(contracts).to.have.lengthOf(1);
      expect(contracts[0]).to.deep.equal(dataContract);
    });
  });

  describe('#getDataContracts', () => {
    it('should get data contracts', async () => {
      blockExecutionState.addDataContract(dataContract);
      blockExecutionState.addDataContract(dataContract);

      const contracts = blockExecutionState.getDataContracts();

      expect(contracts).to.have.lengthOf(2);
      expect(contracts[0]).to.deep.equal(dataContract);
      expect(contracts[1]).to.deep.equal(dataContract);
    });
  });

  describe('#getAccumulativeFees', () => {
    it('should get accumulative fees', async () => {
      let result = blockExecutionState.getAccumulativeFees();

      expect(result).to.equal(0);

      blockExecutionState.accumulativeFees = 10;

      result = blockExecutionState.getAccumulativeFees();

      expect(result).to.equal(10);
    });
  });

  describe('#incrementAccumulativeFees', () => {
    it('should increment accumulative fees', async () => {
      let result = blockExecutionState.getAccumulativeFees();

      expect(result).to.equal(0);

      blockExecutionState.incrementAccumulativeFees(15);

      result = blockExecutionState.getAccumulativeFees();

      expect(result).to.equal(15);
    });
  });

  describe('#reset', () => {
    it('should reset state', () => {
      blockExecutionState.addDataContract(dataContract);

      expect(blockExecutionState.getDataContracts()).to.have.lengthOf(1);

      blockExecutionState.reset();

      expect(blockExecutionState.getDataContracts()).to.have.lengthOf(0);

      expect(blockExecutionState.getHeader()).to.be.null();
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

      const result = blockExecutionState.setHeader(header);

      expect(result).to.equal(blockExecutionState);

      expect(blockExecutionState.header).to.deep.equal(header);
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

      blockExecutionState.header = header;

      expect(blockExecutionState.getHeader()).to.deep.equal(header);
    });
  });
});
