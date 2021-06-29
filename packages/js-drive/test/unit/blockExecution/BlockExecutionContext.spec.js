const {
  tendermint: {
    abci: {
      LastCommitInfo,
    },
    types: {
      Header,
    },
  },
} = require('@dashevo/abci/types');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const BlockExecutionContext = require('../../../lib/blockExecution/BlockExecutionContext');
const getBlockExecutionContextObjectFixture = require('../../../lib/test/fixtures/getBlockExecutionContextObjectFixture');

describe('BlockExecutionContext', () => {
  let blockExecutionContext;
  let dataContract;
  let lastCommitInfo;
  let header;
  let logger;
  let cumulativeFees;
  let plainObject;
  let validTxs;
  let invalidTxs;

  beforeEach(() => {
    blockExecutionContext = new BlockExecutionContext();
    dataContract = getDataContractFixture();
    delete dataContract.entropy;

    plainObject = getBlockExecutionContextObjectFixture(dataContract);

    lastCommitInfo = LastCommitInfo.fromObject(plainObject.lastCommitInfo);

    header = Header.fromObject(plainObject.header);

    logger = plainObject.consensusLogger;
    cumulativeFees = plainObject.cumulativeFees;
    validTxs = plainObject.validTxs;
    invalidTxs = plainObject.invalidTxs;
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

      blockExecutionContext.cumulativeFees = cumulativeFees;

      result = blockExecutionContext.getCumulativeFees();

      expect(result).to.equal(cumulativeFees);
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
      const result = blockExecutionContext.setHeader(header);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.header).to.deep.equal(header);
    });
  });

  describe('#getHeader', () => {
    it('should get header', async () => {
      blockExecutionContext.header = header;

      expect(blockExecutionContext.getHeader()).to.deep.equal(header);
    });
  });

  describe('#setLastCommitInfo', () => {
    it('should set lastCommitInfo', async () => {
      const result = blockExecutionContext.setLastCommitInfo(lastCommitInfo);

      expect(result).to.equal(blockExecutionContext);

      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
    });
  });

  describe('#getLastCommitInfo', () => {
    it('should get lastCommitInfo', async () => {
      blockExecutionContext.lastCommitInfo = lastCommitInfo;

      expect(blockExecutionContext.getLastCommitInfo()).to.deep.equal(lastCommitInfo);
    });
  });

  describe('#populate', () => {
    it('should populate instance from another instance', () => {
      const anotherBlockExecutionContext = new BlockExecutionContext();

      anotherBlockExecutionContext.dataContracts = [dataContract];
      anotherBlockExecutionContext.lastCommitInfo = lastCommitInfo;
      anotherBlockExecutionContext.cumulativeFees = cumulativeFees;
      anotherBlockExecutionContext.header = header;
      anotherBlockExecutionContext.validTxs = validTxs;
      anotherBlockExecutionContext.invalidTxs = invalidTxs;
      anotherBlockExecutionContext.consensusLogger = logger;

      blockExecutionContext.populate(anotherBlockExecutionContext);

      expect(blockExecutionContext.dataContracts).to.equal(
        anotherBlockExecutionContext.dataContracts,
      );
      expect(blockExecutionContext.lastCommitInfo).to.equal(
        anotherBlockExecutionContext.lastCommitInfo,
      );
      expect(blockExecutionContext.cumulativeFees).to.equal(
        anotherBlockExecutionContext.cumulativeFees,
      );
      expect(blockExecutionContext.header).to.equal(
        anotherBlockExecutionContext.header,
      );
      expect(blockExecutionContext.validTxs).to.equal(
        anotherBlockExecutionContext.validTxs,
      );
      expect(blockExecutionContext.invalidTxs).to.equal(
        anotherBlockExecutionContext.invalidTxs,
      );
      expect(blockExecutionContext.consensusLogger).to.equal(
        anotherBlockExecutionContext.consensusLogger,
      );
    });
  });

  describe('#toObject', () => {
    it('should return a plain object', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeFees = cumulativeFees;
      blockExecutionContext.header = header;
      blockExecutionContext.validTxs = validTxs;
      blockExecutionContext.invalidTxs = invalidTxs;
      blockExecutionContext.consensusLogger = logger;

      expect(blockExecutionContext.toObject()).to.deep.equal(plainObject);
    });

    it('should skipConsensusLogger if the option passed', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeFees = cumulativeFees;
      blockExecutionContext.header = header;
      blockExecutionContext.validTxs = validTxs;
      blockExecutionContext.invalidTxs = invalidTxs;
      blockExecutionContext.consensusLogger = logger;

      const result = blockExecutionContext.toObject({ skipConsensusLogger: true });

      delete plainObject.consensusLogger;

      expect(result).to.deep.equal(plainObject);
    });
  });

  describe('#fromObject', () => {
    it('should populate instance from a plain object', () => {
      blockExecutionContext.fromObject(plainObject);

      expect(blockExecutionContext.dataContracts).to.deep.equal([dataContract]);
      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
      expect(blockExecutionContext.cumulativeFees).to.equal(cumulativeFees);
      expect(blockExecutionContext.header).to.deep.equal(header);
      expect(blockExecutionContext.validTxs).to.equal(validTxs);
      expect(blockExecutionContext.invalidTxs).to.equal(invalidTxs);
      expect(blockExecutionContext.consensusLogger).to.equal(logger);
    });
  });
});
