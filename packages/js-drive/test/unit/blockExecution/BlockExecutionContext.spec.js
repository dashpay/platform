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
  let cumulativeProcessingFee;
  let cumulativeStorageFee;
  let plainObject;
  let validTxs;
  let invalidTxs;
  let epochInfo;

  beforeEach(() => {
    blockExecutionContext = new BlockExecutionContext();
    dataContract = getDataContractFixture();
    delete dataContract.entropy;

    plainObject = getBlockExecutionContextObjectFixture(dataContract);

    lastCommitInfo = LastCommitInfo.fromObject(plainObject.lastCommitInfo);

    header = Header.fromObject(plainObject.header);

    logger = plainObject.consensusLogger;
    cumulativeProcessingFee = plainObject.cumulativeProcessingFee;
    cumulativeStorageFee = plainObject.cumulativeStorageFee;
    validTxs = plainObject.validTxs;
    invalidTxs = plainObject.invalidTxs;
    epochInfo = plainObject.epochInfo;
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

  describe('#getCumulativeProcessingFee', () => {
    it('should get cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(0);

      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;

      result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(cumulativeProcessingFee);
    });
  });

  describe('#getCumulativeStorageFee', () => {
    it('should get cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(0);

      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;

      result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(cumulativeStorageFee);
    });
  });

  describe('#incrementCumulativeProcessingFee', () => {
    it('should increment cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(0);

      blockExecutionContext.incrementCumulativeProcessingFee(15);

      result = blockExecutionContext.getCumulativeProcessingFee();

      expect(result).to.equal(15);
    });
  });

  describe('#incrementCumulativeStorageFee', () => {
    it('should increment cumulative fees', async () => {
      let result = blockExecutionContext.getCumulativeStorageFee();

      expect(result).to.equal(0);

      blockExecutionContext.incrementCumulativeStorageFee(15);

      result = blockExecutionContext.getCumulativeStorageFee();

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

  describe('#setEpochInfo', () => {
    it('should set epoch info');
  });

  describe('#getEpochInfo', () => {
    it('should return epoch info');
  });

  describe('#createBlockInfo', () => {
    it('should create block info');
    it('should throw an error if header is not present');
    it('should throw an error if epoch info in not present');
  });

  describe('#populate', () => {
    it('should populate instance from another instance', () => {
      const anotherBlockExecutionContext = new BlockExecutionContext();

      anotherBlockExecutionContext.dataContracts = [dataContract];
      anotherBlockExecutionContext.lastCommitInfo = lastCommitInfo;
      anotherBlockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      anotherBlockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      anotherBlockExecutionContext.header = header;
      anotherBlockExecutionContext.validTxs = validTxs;
      anotherBlockExecutionContext.invalidTxs = invalidTxs;
      anotherBlockExecutionContext.consensusLogger = logger;
      anotherBlockExecutionContext.epochInfo = epochInfo;

      blockExecutionContext.populate(anotherBlockExecutionContext);

      expect(blockExecutionContext.dataContracts).to.equal(
        anotherBlockExecutionContext.dataContracts,
      );
      expect(blockExecutionContext.lastCommitInfo).to.equal(
        anotherBlockExecutionContext.lastCommitInfo,
      );
      expect(blockExecutionContext.cumulativeProcessingFee).to.equal(
        anotherBlockExecutionContext.cumulativeProcessingFee,
      );
      expect(blockExecutionContext.cumulativeStorageFee).to.equal(
        anotherBlockExecutionContext.cumulativeStorageFee,
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
      expect(blockExecutionContext.epochInfo).to.equal(
        anotherBlockExecutionContext.epochInfo,
      );
    });
  });

  describe('#toObject', () => {
    it('should return a plain object', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      blockExecutionContext.header = header;
      blockExecutionContext.validTxs = validTxs;
      blockExecutionContext.invalidTxs = invalidTxs;
      blockExecutionContext.consensusLogger = logger;
      blockExecutionContext.epochInfo = epochInfo;

      expect(blockExecutionContext.toObject()).to.deep.equal(plainObject);
    });

    it('should skipConsensusLogger if the option passed', () => {
      blockExecutionContext.dataContracts = [dataContract];
      blockExecutionContext.lastCommitInfo = lastCommitInfo;
      blockExecutionContext.cumulativeProcessingFee = cumulativeProcessingFee;
      blockExecutionContext.cumulativeStorageFee = cumulativeStorageFee;
      blockExecutionContext.header = header;
      blockExecutionContext.validTxs = validTxs;
      blockExecutionContext.invalidTxs = invalidTxs;
      blockExecutionContext.consensusLogger = logger;
      blockExecutionContext.epochInfo = epochInfo;

      const result = blockExecutionContext.toObject({ skipConsensusLogger: true });

      delete plainObject.consensusLogger;

      expect(result).to.deep.equal(plainObject);
    });
  });

  describe('#fromObject', () => {
    it('should populate instance from a plain object', () => {
      blockExecutionContext.fromObject(plainObject);

      if (blockExecutionContext.dataContracts[0].$defs === undefined) {
        blockExecutionContext.dataContracts[0].$defs = {};
      }

      expect(blockExecutionContext.dataContracts).to.have.deep.members(
        [dataContract],
      );
      expect(blockExecutionContext.lastCommitInfo).to.deep.equal(lastCommitInfo);
      expect(blockExecutionContext.cumulativeProcessingFee).to.equal(cumulativeProcessingFee);
      expect(blockExecutionContext.cumulativeStorageFee).to.equal(cumulativeStorageFee);
      expect(blockExecutionContext.header).to.deep.equal(header);
      expect(blockExecutionContext.validTxs).to.equal(validTxs);
      expect(blockExecutionContext.invalidTxs).to.equal(invalidTxs);
      expect(blockExecutionContext.consensusLogger).to.equal(logger);
    });
  });
});
