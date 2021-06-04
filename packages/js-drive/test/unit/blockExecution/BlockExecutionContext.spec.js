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

  describe('#setLastCommitInfo', () => {
    it('should set lastCommitInfo', async () => {
      const lastCommitInfo = {
        quorumHash: Uint8Array.from('000003c60ecd9576a05a7e15d93baae18729cb4477d44246093bd2cf8d4f53d8'),
        blockSignature: Uint8Array.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b'),
        stateSignature: Uint8Array.from('09c3e46f5bc1abcb7c130b8c36a168e1fbc471fa86445dfce49e151086a277216e7a5618a7554b823d995c5606d0642f18f9c4caa249605d2ab156e14728c82f58f9008d4bcc6e21e0a561e3185e2ae654605613e86af507ca49079595872532'),
      };

      const result = blockExecutionContext.setLastCommitInfo(lastCommitInfo);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
    });
  });

  describe('#getLastCommitInfo', () => {
    it('should get lastCommitInfo', async () => {
      const lastCommitInfo = {
        quorumHash: Uint8Array.from('000003c60ecd9576a05a7e15d93baae18729cb4477d44246093bd2cf8d4f53d8'),
        blockSignature: Uint8Array.from('003657bb44d74c371d14485117de43313ca5c2848f3622d691c2b1bf3576a64bdc2538efab24854eb82ae7db38482dbd15a1cb3bc98e55173817c9d05c86e47a5d67614a501414aae6dd1565e59422d1d77c41ae9b38de34ecf1e9f778b2a97b'),
        stateSignature: Uint8Array.from('09c3e46f5bc1abcb7c130b8c36a168e1fbc471fa86445dfce49e151086a277216e7a5618a7554b823d995c5606d0642f18f9c4caa249605d2ab156e14728c82f58f9008d4bcc6e21e0a561e3185e2ae654605613e86af507ca49079595872532'),
      };

      blockExecutionContext.lastCommitInfo = lastCommitInfo;

      expect(blockExecutionContext.getLastCommitInfo()).to.deep.equal(lastCommitInfo);
    });
  });
});
